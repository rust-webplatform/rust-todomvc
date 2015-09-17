use std::mem;

pub use self::Token::*;
pub use self::ParserState::*;
pub use self::TokenClass::*;

/// `Token` is a section of a compiled mustache string.
#[derive(Clone, Debug)]
pub enum Token {
    Text(String),
    ETag(Vec<String>, String),
    UTag(Vec<String>, String),
    Section(Vec<String>, bool, Vec<Token>, String, String, String, String, String),
    IncompleteSection(Vec<String>, bool, String, bool),
    Partial(String, String, String),
}

enum TokenClass {
    Normal,
    StandAlone,
    WhiteSpace(String, usize),
    NewLineWhiteSpace(String, usize),
}

/// `Parser` parses a string into a series of `Token`s.
pub struct Parser<'a, T: 'a> {
    reader: &'a mut T,
    ch: Option<char>,
    lookahead: Option<char>,
    line: usize,
    col: usize,
    content: String,
    state: ParserState,
    otag: String,
    ctag: String,
    otag_chars: Vec<char>,
    ctag_chars: Vec<char>,
    tag_position: usize,
    tokens: Vec<Token>,
    partials: Vec<String>,
}

enum ParserState { TEXT, OTAG, TAG, CTAG }

impl<'a, T: Iterator<Item=char>> Parser<'a, T> {
    pub fn new(reader: &'a mut T, otag: &str, ctag: &str) -> Parser<'a, T> {
        let mut parser = Parser {
            reader: reader,
            ch: None,
            lookahead: None,
            line: 1,
            col: 1,
            content: String::new(),
            state: TEXT,
            otag: otag.to_string(),
            ctag: ctag.to_string(),
            otag_chars: otag.chars().collect(),
            ctag_chars: ctag.chars().collect(),
            tag_position: 0,
            tokens: Vec::new(),
            partials: Vec::new(),
        };

        parser.bump();
        parser
    }

    fn bump(&mut self) {
        match self.lookahead.take() {
            None => { self.ch = self.reader.next(); }
            Some(ch) => { self.ch = Some(ch); }
        }

        match self.ch {
            Some(ch) => {
                if ch == '\n' {
                    self.line += 1;
                    self.col = 1;
                } else {
                    self.col += 1;
                }
            }
            None => { }
        }
    }

    fn peek(&mut self) -> Option<char> {
        match self.lookahead {
            None => {
                self.lookahead = self.reader.next();
                self.lookahead
            }
            Some(ch) => Some(ch),
        }
    }

    fn ch_is(&self, ch: char) -> bool {
        match self.ch {
            Some(c) => c == ch,
            None => false,
        }
    }

    /// Parse the template into tokens and a list of partial files.
    pub fn parse(mut self) -> (Vec<Token>, Vec<String>) {
        let mut curly_brace_tag = false;

        loop {
            let ch = match self.ch {
                Some(ch) => ch,
                None => { break; }
            };

            match self.state {
                TEXT => {
                    if ch == self.otag_chars[0] {
                        if self.otag_chars.len() > 1 {
                            self.tag_position = 1;
                            self.state = OTAG;
                        } else {
                            self.add_text();
                            self.state = TAG;
                        }
                    } else {
                        self.content.push(ch);
                    }
                    self.bump();
                }
                OTAG => {
                    if ch == self.otag_chars[self.tag_position] {
                        if self.tag_position == self.otag_chars.len() - 1 {
                            self.add_text();
                            curly_brace_tag = false;
                            self.state = TAG;
                        } else {
                            self.tag_position = self.tag_position + 1;
                        }
                    } else {
                        // We don't have a tag, so add all the tag parts we've seen
                        // so far to the string.
                        self.state = TEXT;
                        self.not_otag();
                        self.content.push(ch);
                    }
                    self.bump();
                }
                TAG => {
                    if self.content.is_empty() && ch == '{' {
                        curly_brace_tag = true;
                        self.content.push(ch);
                        self.bump();
                    } else if curly_brace_tag && ch == '}' {
                        curly_brace_tag = false;
                        self.content.push(ch);
                        self.bump();
                    } else if ch == self.ctag_chars[0] {
                        if self.ctag_chars.len() > 1 {
                            self.tag_position = 1;
                            self.state = CTAG;
                            self.bump();
                        } else {
                            self.add_tag();
                            self.state = TEXT;
                        }
                    } else {
                        self.content.push(ch);
                        self.bump();
                    }
                }
                CTAG => {
                    if ch == self.ctag_chars[self.tag_position] {
                        if self.tag_position == self.ctag_chars.len() - 1 {
                            self.add_tag();
                            self.state = TEXT;
                        } else {
                            self.state = TAG;
                            self.not_ctag();
                            self.content.push(ch);
                            self.bump();
                        }
                    } else {
                        panic!("character {} is not part of CTAG: {}",
                              ch,
                              self.ctag_chars[self.tag_position]);
                    }
                }
            }
        }

        match self.state {
            TEXT => { self.add_text(); }
            OTAG => { self.not_otag(); self.add_text(); }
            TAG => { panic!("unclosed tag"); }
            CTAG => { self.not_ctag(); self.add_text(); }
        }

        // Check that we don't have any incomplete sections.
        for token in self.tokens.iter() {
            match *token {
                IncompleteSection(ref path, _, _, _) => {
                    panic!("Unclosed mustache section {}", path.connect("."));
              }
              _ => {}
            }
        };

        let Parser { tokens, partials, .. } = self;

        (tokens, partials)
    }

    fn add_text(&mut self) {
        if !self.content.is_empty() {
            let mut content = String::new();
            mem::swap(&mut content, &mut self.content);

            self.tokens.push(Text(content.as_slice().to_string()));
        }
    }

    // This function classifies whether or not a token is standalone, or if it
    // has trailing whitespace. It's looking for this pattern:
    //
    //   ("\n" | "\r\n") whitespace* token ("\n" | "\r\n")
    //
    fn classify_token(&mut self) -> TokenClass {
        // Exit early if the next character is not '\n' or '\r\n'.
        match self.ch {
            None => { }
            Some(ch) => {
                if !(ch == '\n' || (ch == '\r' && self.peek() == Some('\n'))) {
                    return Normal;
                }
            }
        }

        match self.tokens.last() {
            // If the last token ends with a newline (or there is no previous
            // token), then this token is standalone.
            None => { StandAlone }

            Some(&IncompleteSection(_, _, _, true)) => { StandAlone }

            Some(&Text(ref s)) if !s.is_empty() => {
                // Look for the last newline character that may have whitespace
                // following it.
                match s.as_slice().rfind(| c:char| c == '\n' || !c.is_whitespace()) {
                    // It's all whitespace.
                    None => {
                        if self.tokens.len() == 1 {
                            WhiteSpace(s.to_string(), 0)
                        } else {
                            Normal
                        }
                    }
                    Some(pos) => {
                        if s.as_slice().char_at(pos) == '\n' {
                            if pos == s.len() - 1 {
                                StandAlone
                            } else {
                                WhiteSpace(s.to_string(), pos + 1)
                            }
                        } else { Normal }
                    }
                }
            }
            Some(_) => Normal,
        }
    }

    fn eat_whitespace(&mut self) -> bool {
        // If the next character is a newline, and the last token ends with a
        // newline and whitespace, clear out the whitespace.

        match self.classify_token() {
            Normal => { false }
            StandAlone => {
                if self.ch_is('\r') { self.bump(); }
                self.bump();
                true
            }
            WhiteSpace(s, pos) | NewLineWhiteSpace(s, pos) => {
                if self.ch_is('\r') { self.bump(); }
                self.bump();

                // Trim the whitespace from the last token.
                self.tokens.pop();
                self.tokens.push(Text(s.as_slice()[0..pos].to_string()));

                true
            }
        }
    }

    fn add_tag(&mut self) {
        self.bump();

        let tag = self.otag.clone() + self.content.as_slice() + self.ctag.as_slice();

        // Move the content to avoid a copy.
        let mut content = String::new();
        mem::swap(&mut content, &mut self.content);
        let len = content.len();
        let content = content.as_slice();

        match content.char_at(0) {
            '!' => {
                // ignore comments
                self.eat_whitespace();
            }
            '&' => {
                let name = &content[1..len];
                let name = self.check_content(name);
                let name = name.as_slice().split_terminator('.')
                    .map(|x| x.to_string())
                    .collect();
                self.tokens.push(UTag(name, tag));
            }
            '{' => {
                if content.ends_with("}") {
                    let name = &content[1..len - 1];
                    let name = self.check_content(name);
                    let name = name.as_slice().split_terminator('.')
                        .map(|x| x.to_string())
                        .collect();
                    self.tokens.push(UTag(name, tag));
                } else { panic!("unbalanced \"{\" in tag"); }
            }
            '#' => {
                let newlined = self.eat_whitespace();

                let name = self.check_content(&content[1..len]);
                let name = name.as_slice().split_terminator('.')
                    .map(|x| x.to_string())
                    .collect();
                self.tokens.push(IncompleteSection(name, false, tag, newlined));
            }
            '^' => {
                let newlined = self.eat_whitespace();

                let name = self.check_content(&content[1..len]);
                let name = name.as_slice().split_terminator('.')
                    .map(|x| x.to_string())
                    .collect();
                self.tokens.push(IncompleteSection(name, true, tag, newlined));
            }
            '/' => {
                self.eat_whitespace();

                let name = self.check_content(&content[1..len]);
                let name = name.as_slice().split_terminator('.')
                    .map(|x| x.to_string())
                    .collect();
                let mut children: Vec<Token> = Vec::new();

                loop {
                    if self.tokens.len() == 0 {
                        panic!("closing unopened section");
                    }

                    let last = self.tokens.pop();

                    match last {
                        Some(IncompleteSection(section_name, inverted, osection, _)) => {
                            children.reverse();

                            // Collect all the children's sources.
                            let mut srcs = Vec::new();
                            for child in children.iter() {
                                match *child {
                                    Text(ref s)
                                    | ETag(_, ref s)
                                    | UTag(_, ref s)
                                    | Partial(_, _, ref s) => {
                                        srcs.push(s.clone())
                                    }
                                    Section(_, _, _, _, ref osection, ref src, ref csection, _) => {
                                        srcs.push(osection.clone());
                                        srcs.push(src.clone());
                                        srcs.push(csection.clone());
                                    }
                                    _ => panic!(),
                                }
                            }

                            if section_name == name {
                                // Cache the combination of all the sources in the
                                // section. It's unfortunate, but we need to do this in
                                // case the user uses a function to instantiate the
                                // tag.
                                let mut src = String::new();
                                for s in srcs.iter() { src.push_str(s.as_slice()); }

                                self.tokens.push(
                                    Section(
                                        name,
                                        inverted,
                                        children,
                                        self.otag.to_string(),
                                        osection,
                                        src.as_slice().to_string(),
                                        tag,
                                        self.ctag.to_string()));
                                break;
                            } else {
                                panic!("Unclosed section");
                            }
                        }
                        _ => { match last {
                            Some(last_token) => {children.push(last_token); }
                            None => ()
                            }
                        }
                    }
                }
            }
            '>' => { self.add_partial(content, tag); }
            '=' => {
                self.eat_whitespace();

                if len > 2usize && content.ends_with("=") {
                    let s = self.check_content(&content[1..len - 1]);

                    fn is_whitespace(c: char) -> bool { c.is_whitespace() }
                    let pos = s.as_slice().find(is_whitespace);
                    let pos = match pos {
                      None => { panic!("invalid change delimiter tag content"); }
                      Some(pos) => { pos }
                    };

                    self.otag = s.as_slice()[0..pos].to_string();
                    self.otag_chars = self.otag.as_slice().chars().collect();

                    let s2 = &s.as_slice()[pos..];
                    let pos = s2.find(| c : char| !c.is_whitespace());
                    let pos = match pos {
                      None => { panic!("invalid change delimiter tag content"); }
                      Some(pos) => { pos }
                    };

                    self.ctag = s2[pos..].to_string();
                    self.ctag_chars = self.ctag.as_slice().chars().collect();
                } else {
                    panic!("invalid change delimiter tag content");
                }
            }
            _ => {
                // If the name is "." then we want the top element, which we represent with
                // an empty name.
                let name = self.check_content(content);
                let name = if name.as_slice() == "." {
                    Vec::new()
                } else {
                    name.as_slice().split_terminator('.')
                        .map(|x| x.to_string())
                        .collect()
                };

                self.tokens.push(ETag(name, tag));
            }
        }
    }

    fn add_partial(&mut self, content: &str, tag: String) {
        let indent = match self.classify_token() {
            Normal => "".to_string(),
            StandAlone => {
                if self.ch_is('\r') { self.bump(); }
                self.bump();
                "".to_string()
            }
            WhiteSpace(s, pos) | NewLineWhiteSpace(s, pos) => {
                if self.ch_is('\r') { self.bump(); }
                self.bump();

                let ws = &s.as_slice()[pos..];

                // Trim the whitespace from the last token.
                self.tokens.pop();
                self.tokens.push(Text(s.as_slice()[0..pos].to_string()));

                ws.to_string()
            }
        };

        // We can't inline the tokens directly as we may have a recursive
        // partial. So instead, we'll cache the partials we used and look them
        // up later.
        let name = &content[1..content.len()];
        let name = self.check_content(name);

        self.tokens.push(Partial(name.to_string(), indent, tag));
        self.partials.push(name);
    }

    fn not_otag(&mut self) {
        for (i, ch) in self.otag_chars.iter().enumerate() {
            if !(i < self.tag_position) {
                break
            }
            self.content.push(*ch);
        }
    }

    fn not_ctag(&mut self) {
        for (i, ch) in self.ctag_chars.iter().enumerate() {
            if !(i < self.tag_position) {
                break
            }
            self.content.push(*ch);
        }
    }

    fn check_content(&self, content: &str) -> String {
        let trimmed = content.trim();
        if trimmed.len() == 0 {
            panic!("empty tag");
        }
        trimmed.to_string()
    }
}
