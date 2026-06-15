use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub enum Token {
    Word(String),
    Pipe,
    RedirectOut,
    RedirectAppend,
    RedirectIn,
    And,
    Or,
    Semi,
    Background,
    RedirectStderr,
}

#[derive(Debug, Clone)]
pub enum Connector {
    And,
    Or,
    Semi,
    Background,
}

#[derive(Debug, Clone)]
pub struct Redirect {
    pub kind: RedirectKind,
    pub file: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RedirectKind {
    Out,
    Append,
    In,
    Stderr,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub args: Vec<String>,
    pub redirects: Vec<Redirect>,
    pub background: bool,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub commands: Vec<Command>,
    pub connector: Option<Connector>,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, &'static str> {
    let mut tokens: Vec<Token> = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;
    let len = chars.len();

    while pos < len {
        if chars[pos] == ' ' || chars[pos] == '\t' || chars[pos] == '\n' { pos += 1; continue; }

        match chars[pos] {
            '|' => {
                if pos + 1 < len && chars[pos + 1] == '|' {
                    tokens.push(Token::Or); pos += 2;
                } else {
                    tokens.push(Token::Pipe); pos += 1;
                }
            }
            '&' => {
                if pos + 1 < len && chars[pos + 1] == '&' {
                    tokens.push(Token::And); pos += 2;
                } else {
                    tokens.push(Token::Background); pos += 1;
                }
            }
            ';' => { tokens.push(Token::Semi); pos += 1; }
            '>' => {
                if pos + 1 < len && chars[pos + 1] == '>' {
                    tokens.push(Token::RedirectAppend); pos += 2;
                } else if pos + 1 < len && chars[pos + 1] == '&' {
                    tokens.push(Token::RedirectStderr); pos += 2;
                } else {
                    tokens.push(Token::RedirectOut); pos += 1;
                }
            }
            '<' => { tokens.push(Token::RedirectIn); pos += 1; }
            '"' | '\'' => {
                let quote = chars[pos];
                pos += 1;
                let mut word = String::new();
                while pos < len && chars[pos] != quote {
                    if quote == '"' && chars[pos] == '\\' && pos + 1 < len {
                        pos += 1;
                        match chars[pos] {
                            '"' | '\\' | '$' | '`' | '\n' => word.push(chars[pos]),
                            c => { word.push('\\'); word.push(c); }
                        }
                    } else {
                        word.push(chars[pos]);
                    }
                    pos += 1;
                }
                if pos >= len { return Err("Unterminated quote"); }
                pos += 1;
                tokens.push(Token::Word(word));
            }
            '#' => { break; }
            _ => {
                let mut word = String::new();
                while pos < len && !chars[pos].is_whitespace()
                    && chars[pos] != '|' && chars[pos] != '&'
                    && chars[pos] != ';' && chars[pos] != '>'
                    && chars[pos] != '<' && chars[pos] != '#'
                {
                    if chars[pos] == '\\' && pos + 1 < len {
                        pos += 1; word.push(chars[pos]); pos += 1;
                    } else if chars[pos] == '$' && pos + 1 < len && chars[pos + 1] == '(' {
                        pos += 2;
                        let mut inner = String::new();
                        let mut depth = 1;
                        while pos < len && depth > 0 {
                            if chars[pos] == '(' { depth += 1; }
                            else if chars[pos] == ')' { depth -= 1; if depth == 0 { break; } }
                            inner.push(chars[pos]);
                            pos += 1;
                        }
                        if depth > 0 { return Err("Unterminated $("); }
                        pos += 1;
                        // Execute sub-shell and capture output
                        let output = crate::executor::capture_output(&inner);
                        word.push_str(&output);
                    } else {
                        word.push(chars[pos]); pos += 1;
                    }
                }
                tokens.push(Token::Word(word));
            }
        }
    }
    Ok(tokens)
}

pub fn parse(tokens: &[Token]) -> Vec<Pipeline> {
    let mut pipelines: Vec<Pipeline> = Vec::new();
    let mut current_pipeline = Pipeline { commands: Vec::new(), connector: None };
    let mut current_cmd = Command { args: Vec::new(), redirects: Vec::new(), background: false };
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Word(w) => current_cmd.args.push(w.clone()),
            Token::RedirectOut => {
                i += 1;
                if i < tokens.len() {
                    if let Token::Word(f) = &tokens[i] {
                        current_cmd.redirects.push(Redirect { kind: RedirectKind::Out, file: f.clone() });
                    }
                }
            }
            Token::RedirectAppend => {
                i += 1;
                if i < tokens.len() {
                    if let Token::Word(f) = &tokens[i] {
                        current_cmd.redirects.push(Redirect { kind: RedirectKind::Append, file: f.clone() });
                    }
                }
            }
            Token::RedirectIn => {
                i += 1;
                if i < tokens.len() {
                    if let Token::Word(f) = &tokens[i] {
                        current_cmd.redirects.push(Redirect { kind: RedirectKind::In, file: f.clone() });
                    }
                }
            }
            Token::RedirectStderr => {
                i += 1;
                if i < tokens.len() {
                    if let Token::Word(f) = &tokens[i] {
                        current_cmd.redirects.push(Redirect { kind: RedirectKind::Stderr, file: f.clone() });
                    }
                }
            }
            Token::Pipe => {
                current_pipeline.commands.push(current_cmd);
                current_cmd = Command { args: Vec::new(), redirects: Vec::new(), background: false };
            }
            Token::And => {
                current_pipeline.commands.push(current_cmd);
                current_cmd = Command { args: Vec::new(), redirects: Vec::new(), background: false };
                current_pipeline.connector = Some(Connector::And);
                pipelines.push(current_pipeline);
                current_pipeline = Pipeline { commands: Vec::new(), connector: None };
            }
            Token::Or => {
                current_pipeline.commands.push(current_cmd);
                current_cmd = Command { args: Vec::new(), redirects: Vec::new(), background: false };
                current_pipeline.connector = Some(Connector::Or);
                pipelines.push(current_pipeline);
                current_pipeline = Pipeline { commands: Vec::new(), connector: None };
            }
            Token::Semi => {
                current_pipeline.commands.push(current_cmd);
                current_cmd = Command { args: Vec::new(), redirects: Vec::new(), background: false };
                current_pipeline.connector = Some(Connector::Semi);
                pipelines.push(current_pipeline);
                current_pipeline = Pipeline { commands: Vec::new(), connector: None };
            }
            Token::Background => {
                current_cmd.background = true;
                current_pipeline.commands.push(current_cmd);
                current_cmd = Command { args: Vec::new(), redirects: Vec::new(), background: false };
                current_pipeline.connector = Some(Connector::Background);
                pipelines.push(current_pipeline);
                current_pipeline = Pipeline { commands: Vec::new(), connector: None };
            }
        }
        i += 1;
    }

    if !current_cmd.args.is_empty() || !current_cmd.redirects.is_empty() {
        current_pipeline.commands.push(current_cmd);
    }
    if !current_pipeline.commands.is_empty() {
        pipelines.push(current_pipeline);
    }

    pipelines
}
