use std::collections::HashMap;
use std::io::ErrorKind::FileNotFound;
use std::io::Read;
use std::fs::File;
use std::str;
use std::path::AsPath;

use parser::{Parser, Token};
use super::Context;

/// `Compiler` is a object that compiles a string into a `Vec<Token>`.
pub struct Compiler<T> {
    ctx: Context,
    reader: T,
    partials: HashMap<String, Vec<Token>>,
    otag: String,
    ctag: String,
}

impl<T: Iterator<Item=char>> Compiler<T> {
    /// Construct a default compiler.
    pub fn new(ctx: Context, reader: T) -> Compiler<T> {
        Compiler {
            ctx: ctx,
            reader: reader,
            partials: HashMap::new(),
            otag: "{{".to_string(),
            ctag: "}}".to_string(),
        }
    }

    /// Construct a default compiler.
    pub fn new_with(ctx: Context,
                    reader: T,
                    partials: HashMap<String, Vec<Token>>,
                    otag: String,
                    ctag: String) -> Compiler<T> {
        Compiler {
            ctx: ctx,
            reader: reader,
            partials: partials,
            otag: otag,
            ctag: ctag,
        }
    }

    /// Compiles a template into a series of tokens.
    pub fn compile(mut self) -> (Vec<Token>, HashMap<String, Vec<Token>>) {
        let (tokens, partials) = {
            let parser = Parser::new(&mut self.reader, self.otag.as_slice(), self.ctag.as_slice());
            parser.parse()
        };

        // Compile the partials if we haven't done so already.
        // for name in partials.into_iter() {
        //     let path = self.ctx.template_path
        //                        .as_path()
        //                        .join(&(name.clone() + "." + self.ctx.template_extension.as_slice()));

        //     if !self.partials.contains_key(&name) {
        //         // Insert a placeholder so we don't recurse off to infinity.
        //         self.partials.insert(name.to_string(), Vec::new());

        //         match File::open(&path) {
        //             Ok(mut file) => {
        //                 let mut contents = vec![];
        //                 file.read_to_end(&mut contents).unwrap();
        //                 let string = match str::from_utf8(contents.as_slice()) {
        //                     Ok(string) => string.to_string(),
        //                     Err(_) => { panic!("Failed to parse file as UTF-8"); }
        //                 };

        //                 let compiler = Compiler {
        //                     ctx: self.ctx.clone(),
        //                     reader: string.as_slice().chars(),
        //                     partials: self.partials.clone(),
        //                     otag: "{{".to_string(),
        //                     ctag: "}}".to_string(),
        //                 };

        //                 let (tokens, _) = compiler.compile();

        //                 self.partials.insert(name, tokens);
        //             },
        //             Err(e) => {
        //                 // Ignore missing files.
        //                 if e.kind() != FileNotFound {
        //                     panic!("error reading file: {}", e);
        //                 }
        //             }
        //         }
        //     }
        // }

        let Compiler { partials, .. } = self;

        (tokens, partials)
    }
}

#[cfg(test)]
mod tests {
    use parser::{Token, Text, ETag, UTag, Section, IncompleteSection, Partial};
    use super::Compiler;
    use super::super::Context;
    use std::path::PathBuf;

    fn compile_str(template: &str) -> Vec<Token> {
        let ctx = Context::new(PathBuf::new("."));
        let (tokens, _) = Compiler::new(ctx, template.chars()).compile();
        tokens
    }

    fn token_to_str(token: &Token) -> String {
        match *token {
            // recursive enums crash %?
            Section(ref name,
                    inverted,
                    ref children,
                    ref otag,
                    ref osection,
                    ref src,
                    ref tag,
                    ref ctag) => {
                let name = name.iter().map(|e| format!("{}", *e)).collect::<Vec<String>>();
                let children = children.iter().map(|x| token_to_str(x)).collect::<Vec<String>>();
                format!("Section(vec!({}), {}, vec!({}), {}, {}, {}, {}, {})",
                        name.connect(", "),
                        inverted,
                        children.connect(", "),
                        otag,
                        osection,
                        src,
                        tag,
                        ctag)
            }
            ETag(ref name, ref tag) => {
                let name = name.iter().map(|e| format!("{}", *e)).collect::<Vec<String>>();
                format!("ETag(vec!({}), {})", name.connect(", "), *tag)
            }
            UTag(ref name, ref tag) => {
                let name = name.iter().map(|e| format!("{}", *e)).collect::<Vec<String>>();
                format!("UTag(vec!({}), {})", name.connect(", "), *tag)
            }
            IncompleteSection(ref name, ref inverted, ref osection, ref newlined) => {
                let name = name.iter().map(|e| format!("{}", *e)).collect::<Vec<String>>();
                format!("IncompleteSection(vec!({}), {}, {}, {})",
                        name.connect(", "),
                        *inverted,
                        *osection,
                        *newlined)
            }
            _ => {
                format!("{:?}", token)
            }
        }
    }

    fn check_tokens(_actual: Vec<Token>, _expected: &[Token]) {
        // TODO: equality is currently broken for enums
        //let actual: Vec<String> = actual.iter().map(token_to_str).collect();
        //let expected = expected.iter().map(token_to_str).collect();

        //assert_eq!(actual, expected);
    }

    #[test]
    fn test_compile_texts() {
        check_tokens(compile_str("hello world"), &[
            Text("hello world".to_string())
        ]);
        check_tokens(compile_str("hello {world"), &[
            Text("hello {world".to_string())
        ]);
        check_tokens(compile_str("hello world}"), &[
            Text("hello world}".to_string())
        ]);
        check_tokens(compile_str("hello world}}"), &[
            Text("hello world}}".to_string())
        ]);
    }

    #[test]
    fn test_compile_etags() {
        check_tokens(compile_str("{{ name }}"), &[
            ETag(vec!("name".to_string()), "{{ name }}".to_string())
        ]);

        check_tokens(compile_str("before {{name}} after"), &[
            Text("before ".to_string()),
            ETag(vec!("name".to_string()), "{{name}}".to_string()),
            Text(" after".to_string())
        ]);

        check_tokens(compile_str("before {{name}}"), &[
            Text("before ".to_string()),
            ETag(vec!("name".to_string()), "{{name}}".to_string())
        ]);

        check_tokens(compile_str("{{name}} after"), &[
            ETag(vec!("name".to_string()), "{{name}}".to_string()),
            Text(" after".to_string())
        ]);
    }

    #[test]
    fn test_compile_utags() {
        check_tokens(compile_str("{{{name}}}"), &[
            UTag(vec!("name".to_string()), "{{{name}}}".to_string())
        ]);

        check_tokens(compile_str("before {{{name}}} after"), &[
            Text("before ".to_string()),
            UTag(vec!("name".to_string()), "{{{name}}}".to_string()),
            Text(" after".to_string())
        ]);

        check_tokens(compile_str("before {{{name}}}"), &[
            Text("before ".to_string()),
            UTag(vec!("name".to_string()), "{{{name}}}".to_string())
        ]);

        check_tokens(compile_str("{{{name}}} after"), &[
            UTag(vec!("name".to_string()), "{{{name}}}".to_string()),
            Text(" after".to_string())
        ]);
    }

    #[test]
    fn test_compile_sections() {
        check_tokens(compile_str("{{# name}}{{/name}}"), &[
            Section(
                vec!("name".to_string()),
                false,
                Vec::new(),
                "{{".to_string(),
                "{{# name}}".to_string(),
                "".to_string(),
                "{{/name}}".to_string(),
                "}}".to_string()
            )
        ]);

        check_tokens(compile_str("before {{^name}}{{/name}} after"), &[
            Text("before ".to_string()),
            Section(
                vec!("name".to_string()),
                true,
                Vec::new(),
                "{{".to_string(),
                "{{^name}}".to_string(),
                "".to_string(),
                "{{/name}}".to_string(),
                "}}".to_string()
            ),
            Text(" after".to_string())
        ]);

        check_tokens(compile_str("before {{#name}}{{/name}}"), &[
            Text("before ".to_string()),
            Section(
                vec!("name".to_string()),
                false,
                Vec::new(),
                "{{".to_string(),
                "{{#name}}".to_string(),
                "".to_string(),
                "{{/name}}".to_string(),
                "}}".to_string()
            )
        ]);

        check_tokens(compile_str("{{#name}}{{/name}} after"), &[
            Section(
                vec!("name".to_string()),
                false,
                Vec::new(),
                "{{".to_string(),
                "{{#name}}".to_string(),
                "".to_string(),
                "{{/name}}".to_string(),
                "}}".to_string()
            ),
            Text(" after".to_string())
        ]);

        check_tokens(compile_str(
                "before {{#a}} 1 {{^b}} 2 {{/b}} {{/a}} after"), &[
            Text("before ".to_string()),
            Section(
                vec!("a".to_string()),
                false,
                vec!(
                    Text(" 1 ".to_string()),
                    Section(
                        vec!("b".to_string()),
                        true,
                        vec!(Text(" 2 ".to_string())),
                        "{{".to_string(),
                        "{{^b}}".to_string(),
                        " 2 ".to_string(),
                        "{{/b}}".to_string(),
                        "}}".to_string()
                    ),
                    Text(" ".to_string())
                ),
                "{{".to_string(),
                "{{#a}}".to_string(),
                " 1 {{^b}} 2 {{/b}} ".to_string(),
                "{{/a}}".to_string(),
                "}}".to_string()
            ),
            Text(" after".to_string())
        ]);
    }

    #[test]
    fn test_compile_partials() {
        check_tokens(compile_str("{{> test}}"), &[
            Partial("test".to_string(), "".to_string(), "{{> test}}".to_string())
        ]);

        check_tokens(compile_str("before {{>test}} after"), &[
            Text("before ".to_string()),
            Partial("test".to_string(), "".to_string(), "{{>test}}".to_string()),
            Text(" after".to_string())
        ]);

        check_tokens(compile_str("before {{> test}}"), &[
            Text("before ".to_string()),
            Partial("test".to_string(), "".to_string(), "{{> test}}".to_string())
        ]);

        check_tokens(compile_str("{{>test}} after"), &[
            Partial("test".to_string(), "".to_string(), "{{>test}}".to_string()),
            Text(" after".to_string())
        ]);
    }

    #[test]
    fn test_compile_delimiters() {
        check_tokens(compile_str("before {{=<% %>=}}<%name%> after"), &[
            Text("before ".to_string()),
            ETag(vec!("name".to_string()), "<%name%>".to_string()),
            Text(" after".to_string())
        ]);
    }
}
