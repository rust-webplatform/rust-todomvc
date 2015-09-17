use std::io::Write;
use std::collections::HashMap;
use std::mem;
use std::str;
use std::path::AsPath;
use rustc_serialize::Encodable;

use encoder;
use compiler::Compiler;
use parser::Token;
use parser::Token::*;
use encoder::Error;

use super::{Context, Data, Bool, StrVal, VecVal, Map, Fun};

/// `Template` represents a compiled mustache file.
#[derive(Debug, Clone)]
pub struct Template {
    ctx: Context,
    tokens: Vec<Token>,
    partials: HashMap<String, Vec<Token>>
}

/// Construct a `Template`. This is not part of the impl of Template so it is
/// not exported outside of mustache.
pub fn new(ctx: Context, tokens: Vec<Token>, partials: HashMap<String,
Vec<Token>>) -> Template {
    Template {
        ctx: ctx,
        tokens: tokens,
        partials: partials,
    }
}

impl Template {
    /// Renders the template with the `Encodable` data.
    pub fn render<'a, W: Write, T: Encodable>(
        &self,
        wr: &mut W,
        data: &T
    ) -> Result<(), Error> {
        let data = try!(encoder::encode(data));
        Ok(self.render_data(wr, &data))
    }

    /// Renders the template with the `Data`.
    pub fn render_data<'a, W: Write>(&self, wr: &mut W, data: &Data) {
        let mut render_ctx = RenderContext::new(self);
        let mut stack = vec!(data);

        render_ctx.render(
            wr,
            &mut stack,
            self.tokens.as_slice());
    }
}

struct RenderContext<'a> {
    template: &'a Template,
    indent: String,
}

impl<'a> RenderContext<'a> {
    fn new(template: &'a Template) -> RenderContext<'a> {
        RenderContext {
            template: template,
            indent: "".to_string(),
        }
    }

    fn render<'b, W: Write>(
        &mut self,
        wr: &mut W,
        stack: &mut Vec<&Data>,
        tokens: &[Token]
    ) {
        for token in tokens.iter() {
            self.render_token(wr, stack, token);
        }
    }

    fn render_token<'b, W: Write>(
        &mut self,
        wr: &mut W,
        stack: &mut Vec<&Data>,
        token: &Token
    ) {
        match *token {
            Text(ref value) => {
                self.render_text(wr, value.as_slice());
            },
            ETag(ref path, _) => {
                self.render_etag(wr, stack, path.as_slice());
            }
            UTag(ref path, _) => {
                self.render_utag(wr, stack, path.as_slice());
            }
            Section(ref path, true, ref children, _, _, _, _, _) => {
                self.render_inverted_section(wr, stack, path.as_slice(), children.as_slice());
            }
            Section(ref path, false, ref children, ref otag, _, ref src, _, ref ctag) => {
                self.render_section(
                    wr,
                    stack,
                    path.as_slice(),
                    children.as_slice(),
                    src.as_slice(),
                    otag.as_slice(),
                    ctag.as_slice())
            }
            Partial(ref name, ref indent, _) => {
                self.render_partial(wr, stack, name.as_slice(), indent.as_slice());
            }
            _ => { panic!() }
        }
    }

    fn render_text<W: Write>(
        &mut self,
        wr: &mut W,
        value: &str
    ) {
        // Indent the lines.
        if self.indent.is_empty() {
            wr.write_all(value.as_bytes()).unwrap();
        } else {
            let mut pos = 0;
            let len = value.len();

            while pos < len {
                let v = &value[pos..];
                let line = match v.find('\n') {
                    None => {
                        let line = v;
                        pos = len;
                        line
                    }
                    Some(i) => {
                        let line = &v[..i + 1];
                        pos += i + 1;
                        line
                    }
                };

                if line.char_at(0) != '\n' {
                    wr.write_all(self.indent.as_bytes()).unwrap();
                }

                wr.write_all(line.as_bytes()).unwrap();
            }
        }
    }

    fn render_etag<'b, W: Write>(
        &mut self,
        wr: &mut W,
        stack: &mut Vec<&Data>,
        path: &[String]
    ) {
        let mut bytes = vec![];

        self.render_utag(&mut bytes, stack, path);

        let s = str::from_utf8(&bytes).unwrap();

        for b in s.bytes() {
            match b {
                b'<'  => { wr.write_all(b"&lt;").unwrap(); }
                b'>'  => { wr.write_all(b"&gt;").unwrap(); }
                b'&'  => { wr.write_all(b"&amp;").unwrap(); }
                b'"'  => { wr.write_all(b"&quot;").unwrap(); }
                b'\'' => { wr.write_all(b"&#39;").unwrap(); }
                _    => { wr.write_all(&[b]).unwrap(); }
            }
        }
    }

    fn render_utag<'b, W: Write>(
        &mut self,
        wr: &mut W,
        stack: &mut Vec<&Data>,
        path: &[String]
    ) {
        match self.find(path, stack) {
            None => { }
            Some(value) => {
                wr.write_all(self.indent.as_bytes()).unwrap();

                match *value {
                    StrVal(ref value) => {
                        wr.write_all(value.as_bytes()).unwrap();
                    }

                    // etags and utags use the default delimiter.
                    Fun(ref fcell) => {
                        let f = &mut *fcell.borrow_mut();
                        let tokens = self.render_fun("", "{{", "}}", f);
                        self.render(wr, stack, tokens.as_slice());
                    }

                    ref value => { panic!("unexpected value {:?}", value); }
                }
            }
        };
    }

    fn render_inverted_section<'b, W: Write>(
        &mut self,
        wr: &mut W,
        stack: &mut Vec<&Data>,
        path: &[String],
        children: &[Token]
    ) {
        match self.find(path, stack) {
            None => { }
            Some(&Bool(false)) => { }
            Some(&VecVal(ref xs)) if xs.is_empty() => { }
            Some(_) => { return; }
        }

        self.render(wr, stack, children);
    }

    fn render_section<'b, W: Write>(
        &mut self,
        wr: &mut W,
        stack: &mut Vec<&Data>,
        path: &[String],
        children: &[Token],
        src: &str,
        otag: &str,
        ctag: &str
    ) {
        match self.find(path, stack) {
            None => { }
            Some(value) => {
                match *value {
                    Bool(true) => {
                        self.render(wr, stack, children);
                    }
                    Bool(false) => { }
                    VecVal(ref vs) => {
                        for v in vs.iter() {
                            stack.push(v);
                            self.render(wr, stack, children);
                            stack.pop();
                        }
                    }
                    Map(_) => {
                        stack.push(value);
                        self.render(wr, stack, children);
                        stack.pop();
                    }
                    Fun(ref fcell) => {
                        let f = &mut *fcell.borrow_mut();
                        let tokens = self.render_fun(src, otag, ctag, f);
                        self.render(wr, stack, tokens.as_slice())
                    }
                    _ => { panic!("unexpected value {:?}", value) }
                }
            }
        }
    }

    fn render_partial<'b, W: Write>(
        &mut self,
        wr: &mut W,
        stack: &mut Vec<&Data>,
        name: &str,
        indent: &str
    ) {
        match self.template.partials.get(name) {
            None => { }
            Some(ref tokens) => {
                let mut indent = self.indent.clone() + indent;

                mem::swap(&mut self.indent, &mut indent);
                self.render(wr, stack, tokens.as_slice());
                mem::swap(&mut self.indent, &mut indent);
            }
        }
    }

    fn render_fun(&self, src: &str, otag: &str, ctag: &str, f: &mut Box<FnMut(String) -> String + Send + 'static>) -> Vec<Token> {
        let src = f(src.to_string());

        let compiler = Compiler::new_with(
            self.template.ctx.clone(),
            src.as_slice().chars(),
            self.template.partials.clone(),
            otag.to_string(),
            ctag.to_string());

        let (tokens, _) = compiler.compile();
        tokens
    }

    fn find<'b, 'c>(&self, path: &[String], stack: &mut Vec<&'c Data>) -> Option<&'c Data> {
        // If we have an empty path, we just want the top value in our stack.
        if path.is_empty() {
            match stack.last() {
                None => { return None; }
                Some(data) => { return Some(*data); }
            }
        }

        // Otherwise, find the stack that has the first part of our path.
        let mut value = None;

        for data in stack.iter().rev() {
            match **data {
                Map(ref m) => {
                    match m.get(&path[0]) {
                        Some(v) => {
                            value = Some(v);
                            break;
                        }
                        None => { }
                    }
                }
                _ => { panic!("expect map: {:?}", path) }
            }
        }

        // Walk the rest of the path to find our final value.
        let mut value = match value {
            Some(value) => value,
            None => { return None; }
        };

        for part in path[1..].iter() {
            match *value {
                Map(ref m) => {
                    match m.get(part) {
                        Some(v) => { value = v; }
                        None => { return None; }
                    }
                }
                _ => { return None; }
            }
        }

        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs::{File, TempDir};
    use std::io::{Read, Write};
    use std::path::{PathBuf, Path, AsPath};
    use std::collections::HashMap;
    use rustc_serialize::{json, Encodable};
    use rustc_serialize::json::Json;

    use encoder::{Encoder, Error};

    use super::super::compile_str;
    use super::super::{Data, StrVal, VecVal, Map, Fun};
    use super::super::{Context, Template};

    #[derive(RustcEncodable)]
    struct Name { name: String }

    fn render<'a, 'b, T: Encodable>(
        template: &str,
        data: &T,
    ) -> Result<String, Error> {
        let template = compile_str(template);

        let mut bytes = vec![];
        try!(template.render(&mut bytes, data));

        Ok(String::from_utf8(bytes).unwrap())
    }

    #[test]
    fn test_render_texts() {
        let ctx = Name { name: "world".to_string() };

        assert_eq!(render("hello world", &ctx), Ok("hello world".to_string()));
        assert_eq!(render("hello {world", &ctx), Ok("hello {world".to_string()));
        assert_eq!(render("hello world}", &ctx), Ok("hello world}".to_string()));
        assert_eq!(render("hello {world}", &ctx), Ok("hello {world}".to_string()));
        assert_eq!(render("hello world}}", &ctx), Ok("hello world}}".to_string()));
    }

    #[test]
    fn test_render_etags() {
        let ctx = Name { name: "world".to_string() };

        assert_eq!(render("hello {{name}}", &ctx), Ok("hello world".to_string()));
    }

    #[test]
    fn test_render_utags() {
        let ctx = Name { name: "world".to_string() };

        assert_eq!(render("hello {{{name}}}", &ctx), Ok("hello world".to_string()));
    }

    fn render_data(template: &Template, data: &Data) -> String {
        let mut bytes = vec![];
        template.render_data(&mut bytes, data);
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn test_render_sections() {
        let ctx = HashMap::new();
        let template = compile_str("0{{#a}}1 {{n}} 3{{/a}}5");

        assert_eq!(render_data(&template, &Map(ctx)), "05".to_string());

        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), VecVal(Vec::new()));

        assert_eq!(render_data(&template, &Map(ctx)), "05".to_string());

        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), VecVal(Vec::new()));
        assert_eq!(render_data(&template, &Map(ctx)), "05".to_string());

        let mut ctx0 = HashMap::new();
        let ctx1 = HashMap::new();
        ctx0.insert("a".to_string(), VecVal(vec!(Map(ctx1))));

        assert_eq!(render_data(&template, &Map(ctx0)), "01  35".to_string());

        let mut ctx0 = HashMap::new();
        let mut ctx1 = HashMap::new();
        ctx1.insert("n".to_string(), StrVal("a".to_string()));
        ctx0.insert("a".to_string(), VecVal(vec!(Map(ctx1))));
        assert_eq!(render_data(&template, &Map(ctx0)), "01 a 35".to_string());

        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), Fun(RefCell::new(Box::new(|_text| { "foo".to_string() }))));
        assert_eq!(render_data(&template, &Map(ctx)), "0foo5".to_string());
    }

    #[test]
    fn test_render_inverted_sections() {
        let template = compile_str("0{{^a}}1 3{{/a}}5");

        let ctx = HashMap::new();
        assert_eq!(render_data(&template, &Map(ctx)), "01 35".to_string());

        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), VecVal(vec!()));
        assert_eq!(render_data(&template, &Map(ctx)), "01 35".to_string());

        let mut ctx0 = HashMap::new();
        let ctx1 = HashMap::new();
        ctx0.insert("a".to_string(), VecVal(vec!(Map(ctx1))));
        assert_eq!(render_data(&template, &Map(ctx0)), "05".to_string());

        let mut ctx0 = HashMap::new();
        let mut ctx1 = HashMap::new();
        ctx1.insert("n".to_string(), StrVal("a".to_string()));
        ctx0.insert("a".to_string(), VecVal(vec!(Map(ctx1))));
        assert_eq!(render_data(&template, &Map(ctx0)), "05".to_string());
    }

    #[test]
    fn test_render_partial() {
        let template = Context::new(PathBuf::new("src/test-data"))
            .compile_path(PathBuf::new("base"))
            .ok()
            .unwrap();

        let ctx = HashMap::new();
        assert_eq!(render_data(&template, &Map(ctx)), "<h2>Names</h2>\n".to_string());

        let mut ctx = HashMap::new();
        ctx.insert("names".to_string(), VecVal(vec!()));
        assert_eq!(render_data(&template, &Map(ctx)), "<h2>Names</h2>\n".to_string());

        let mut ctx0 = HashMap::new();
        let ctx1 = HashMap::new();
        ctx0.insert("names".to_string(), VecVal(vec!(Map(ctx1))));
        assert_eq!(
            render_data(&template, &Map(ctx0)),
            "<h2>Names</h2>\n  <strong></strong>\n\n".to_string());

        let mut ctx0 = HashMap::new();
        let mut ctx1 = HashMap::new();
        ctx1.insert("name".to_string(), StrVal("a".to_string()));
        ctx0.insert("names".to_string(), VecVal(vec!(Map(ctx1))));
        assert_eq!(
            render_data(&template, &Map(ctx0)),
            "<h2>Names</h2>\n  <strong>a</strong>\n\n".to_string());

        let mut ctx0 = HashMap::new();
        let mut ctx1 = HashMap::new();
        ctx1.insert("name".to_string(), StrVal("a".to_string()));
        let mut ctx2 = HashMap::new();
        ctx2.insert("name".to_string(), StrVal("<b>".to_string()));
        ctx0.insert("names".to_string(), VecVal(vec!(Map(ctx1), Map(ctx2))));
        assert_eq!(
            render_data(&template, &Map(ctx0)),
            "<h2>Names</h2>\n  <strong>a</strong>\n\n  <strong>&lt;b&gt;</strong>\n\n".to_string());
    }

    fn parse_spec_tests(src: &str) -> Vec<json::Json> {
        let path = PathBuf::new(src);
        let mut file_contents = vec![];
        match File::open(&path).and_then(|mut f| f.read_to_end(&mut file_contents)) {
            Ok(()) => {},
            Err(e) => panic!("Could not read file {}", e),
        };

        let s = String::from_utf8(file_contents.as_slice().to_vec())
                     .ok().expect("File was not UTF8 encoded");

        match Json::from_str(s.as_slice()) {
            Err(e) =>  panic!("{:?}", e),
            Ok(json) => {
                match json {
                    Json::Object(d) => {
                        let mut d = d;
                        match d.remove(&"tests".to_string()) {
                            Some(Json::Array(tests)) => tests.into_iter().collect(),
                            _ => panic!("{}: tests key not a list", src),
                        }
                    }
                    _ => panic!("{}: JSON value not a map", src),
                }
            }
        }
    }

    fn write_partials(tmpdir: &Path, value: &json::Json) {
        match value {
            &Json::Object(ref d) => {
                for (key, value) in d.iter() {
                    match value {
                        &Json::String(ref s) => {
                            let path = tmpdir.join(&(key.clone() + ".mustache"));
                            File::create(&path).and_then(|mut f| f.write_all(s.as_bytes())).unwrap();
                        }
                        _ => panic!(),
                    }
                }
            },
            _ => panic!(),
        }
    }

    fn run_test(test: json::Object, data: Data) {
        let template = match test.get(&"template".to_string()) {
            Some(&Json::String(ref s)) => s.clone(),
            _ => panic!(),
        };

        let expected = match test.get(&"expected".to_string()) {
            Some(&Json::String(ref s)) => s.clone(),
            _ => panic!(),
        };

        // Make a temporary dir where we'll store our partials. This is to
        // avoid a race on filenames.
        let tmpdir = match TempDir::new("") {
            Ok(tmpdir) => tmpdir,
            Err(_) => panic!(),
        };

        match test.get(&"partials".to_string()) {
            Some(value) => write_partials(tmpdir.path(), value),
            None => {},
        }

        let ctx = Context::new(tmpdir.path().to_path_buf());
        let template = ctx.compile(template.as_slice().chars());
        let result = render_data(&template, &data);

        if result != expected {
            println!("desc:     {}", test.get(&"desc".to_string()).unwrap().to_string());
            println!("context:  {}", test.get(&"data".to_string()).unwrap().to_string());
            println!("=>");
            println!("template: {:?}", template);
            println!("expected: {}", expected);
            println!("actual:   {}", result);
            println!("");
        }
        assert_eq!(result, expected);
    }

    fn run_tests(spec: &str) {
        for json in parse_spec_tests(spec).into_iter() {
            let test = match json {
                Json::Object(m) => m,
                _ => panic!(),
            };

            let data = match test.get(&"data".to_string()) {
                Some(data) => data.clone(),
                None => panic!(),
            };

            let mut encoder = Encoder::new();
            data.encode(&mut encoder).ok().unwrap();
            assert_eq!(encoder.data.len(), 1);

            run_test(test, encoder.data.pop().unwrap());
        }
    }

    #[test]
    fn test_spec_comments() {
        run_tests("spec/specs/comments.json");
    }

    #[test]
    fn test_spec_delimiters() {
        run_tests("spec/specs/delimiters.json");
    }

    #[test]
    fn test_spec_interpolation() {
        run_tests("spec/specs/interpolation.json");
    }

    #[test]
    fn test_spec_inverted() {
        run_tests("spec/specs/inverted.json");
    }

    #[test]
    fn test_spec_partials() {
        run_tests("spec/specs/partials.json");
    }

    #[test]
    fn test_spec_sections() {
        run_tests("spec/specs/sections.json");
    }

    #[test]
    fn test_spec_lambdas() {
        for json in parse_spec_tests("spec/specs/~lambdas.json").into_iter() {
            let mut test = match json {
                Json::Object(m) => m,
                value => { panic!("{}", value) }
            };

            let s = match test.remove(&"name".to_string()) {
                Some(Json::String(s)) => s,
                value => { panic!("{:?}", value) }
            };

            // Replace the lambda with rust code.
            let data = match test.remove(&"data".to_string()) {
                Some(data) => data,
                None => panic!(),
            };

            let mut encoder = Encoder::new();
            data.encode(&mut encoder).ok().unwrap();

            let mut ctx = match encoder.data.pop().unwrap() {
                Map(ctx) => ctx,
                _ => panic!(),
            };

            // needed for the closure test.
            let mut calls = 0usize;

            match s.as_slice() {
                "Interpolation" => {
                    let f = |_text| { "world".to_string() };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                "Interpolation - Expansion" => {
                    let f = |_text| { "{{planet}}".to_string() };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                "Interpolation - Alternate Delimiters" => {
                    let f = |_text| { "|planet| => {{planet}}".to_string() };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                "Interpolation - Multiple Calls" => {
                    let f = move |_text: String| {
                        calls += 1;
                        calls.to_string()
                    };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                "Escaping" => {
                    let f = |_text| { ">".to_string() };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                "Section" => {
                    let f = | text: String| {
                        if text.as_slice() == "{{x}}" {
                            "yes".to_string()
                        } else {
                            "no".to_string()
                        }
                    };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                "Section - Expansion" => {
                    let f = | text: String| { text.clone() + "{{planet}}" + text.clone().as_slice() };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                "Section - Alternate Delimiters" => {
                    let f = | text: String| { text.clone() + "{{planet}} => |planet|" + text.clone().as_slice() };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                "Section - Multiple Calls" => {
                    let f = | text: String| { "__".to_string() + text.as_slice() + "__" };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                "Inverted Section" => {
                    let f= |_text| { "".to_string() };
                    ctx.insert("lambda".to_string(), Fun(RefCell::new(Box::new(f))));
                },
                value => { panic!("{}", value) }
            };

            run_test(test, Map(ctx));
        }
    }
}
