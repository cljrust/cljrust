/// Lexer: tokenizes Clojure source text

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Symbol(String),
    String(String),
    Integer(i64),
    Float(f64),
    Char(char),
    Keyword(String),   // :keyword
    Bool(bool),
    Nil,
    Ampersand,         // &
    At,                // @  (deref)
    Hash,              // #
    Arrow,             // ->
    FatArrow,          // =>
    Dot,               // .
    DotDot,            // ..
    DotDotEq,          // ..=
    Tilde,             // ~ (raw rust escape)
    Quote,             // '
}

#[derive(Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<SpannedToken>, String> {
        let mut tokens = Vec::new();
        while self.pos < self.input.len() {
            self.skip_whitespace_and_comments();
            if self.pos >= self.input.len() {
                break;
            }
            let span = Span {
                line: self.line,
                col: self.col,
            };
            let token = self.next_token()?;
            tokens.push(SpannedToken { token, span });
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_ahead(&self, n: usize) -> Option<char> {
        self.input.get(self.pos + n).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            if ch.is_whitespace() || ch == ',' {
                // commas are whitespace in Clojure
                self.advance();
            } else if ch == ';' {
                // line comment
                while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, String> {
        let ch = self.peek().unwrap();

        match ch {
            '(' => {
                self.advance();
                Ok(Token::LParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RParen)
            }
            '[' => {
                self.advance();
                Ok(Token::LBracket)
            }
            ']' => {
                self.advance();
                Ok(Token::RBracket)
            }
            '{' => {
                self.advance();
                Ok(Token::LBrace)
            }
            '}' => {
                self.advance();
                Ok(Token::RBrace)
            }
            '&' => {
                self.advance();
                Ok(Token::Ampersand)
            }
            '@' => {
                self.advance();
                Ok(Token::At)
            }
            '#' => {
                self.advance();
                Ok(Token::Hash)
            }
            '\'' => {
                self.advance();
                Ok(Token::Quote)
            }
            '~' => {
                self.advance();
                Ok(Token::Tilde)
            }
            '.' => {
                self.advance();
                if self.peek() == Some('.') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Ok(Token::DotDotEq)
                    } else {
                        Ok(Token::DotDot)
                    }
                } else if self.peek().map_or(true, |c| c.is_whitespace() || c == ')' || c == ']') {
                    Ok(Token::Dot)
                } else {
                    // .method — return as symbol
                    let mut s = String::from(".");
                    while let Some(c) = self.peek() {
                        if is_symbol_char(c) {
                            s.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    Ok(Token::Symbol(s))
                }
            }
            '"' => self.read_string(),
            '\\' => self.read_char_literal(),
            ':' => self.read_keyword(),
            '-' => {
                if self.peek_ahead(1) == Some('>') {
                    self.advance();
                    self.advance();
                    Ok(Token::Arrow)
                } else if self.peek_ahead(1).map_or(false, |c| c.is_ascii_digit()) {
                    self.read_number()
                } else {
                    self.read_symbol()
                }
            }
            '=' => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    Ok(Token::FatArrow)
                } else {
                    // = as a symbol (used for equality)
                    Ok(Token::Symbol("=".to_string()))
                }
            }
            c if c.is_ascii_digit() => self.read_number(),
            c if is_symbol_start(c) => self.read_symbol(),
            '+' | '*' | '/' | '%' | '<' | '>' | '!' | '^' | '|' => self.read_symbol(),
            _ => Err(format!(
                "Unexpected character '{}' at line {} col {}",
                ch, self.line, self.col
            )),
        }
    }

    fn read_string(&mut self) -> Result<Token, String> {
        self.advance(); // skip opening "
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err("Unterminated string literal".to_string()),
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some('0') => s.push('\0'),
                    Some(c) => {
                        s.push('\\');
                        s.push(c);
                    }
                    None => return Err("Unterminated escape in string".to_string()),
                },
                Some('"') => break,
                Some(c) => s.push(c),
            }
        }
        Ok(Token::String(s))
    }

    fn read_char_literal(&mut self) -> Result<Token, String> {
        self.advance(); // skip backslash
        match self.peek() {
            Some('n') if self.peek_ahead(1).map_or(true, |c| !is_symbol_char(c)) => {
                // Could be \newline
                let mut word = String::new();
                while let Some(c) = self.peek() {
                    if c.is_alphabetic() {
                        word.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                match word.as_str() {
                    "newline" => Ok(Token::Char('\n')),
                    "n" => Ok(Token::Char('n')),
                    _ => Ok(Token::Char(word.chars().next().unwrap())),
                }
            }
            Some(c) => {
                self.advance();
                // Check for named chars
                if c.is_alphabetic() {
                    let mut word = String::from(c);
                    while let Some(nc) = self.peek() {
                        if nc.is_alphabetic() {
                            word.push(nc);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    match word.as_str() {
                        "space" => Ok(Token::Char(' ')),
                        "tab" => Ok(Token::Char('\t')),
                        "newline" => Ok(Token::Char('\n')),
                        "return" => Ok(Token::Char('\r')),
                        s if s.len() == 1 => Ok(Token::Char(s.chars().next().unwrap())),
                        _ => Err(format!("Unknown character literal: \\{}", word)),
                    }
                } else {
                    Ok(Token::Char(c))
                }
            }
            None => Err("Unexpected end of input after \\".to_string()),
        }
    }

    fn read_keyword(&mut self) -> Result<Token, String> {
        self.advance(); // skip :
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if is_symbol_char(c) || c == '/' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err(format!("Empty keyword at line {} col {}", self.line, self.col));
        }
        Ok(Token::Keyword(name))
    }

    fn read_number(&mut self) -> Result<Token, String> {
        let mut s = String::new();
        let mut is_float = false;

        if self.peek() == Some('-') {
            s.push('-');
            self.advance();
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                s.push(c);
                self.advance();
            } else if c == '.' && !is_float && self.peek_ahead(1).map_or(false, |nc| nc.is_ascii_digit()) {
                is_float = true;
                s.push(c);
                self.advance();
            } else if c == 'e' || c == 'E' {
                is_float = true;
                s.push(c);
                self.advance();
                if self.peek() == Some('+') || self.peek() == Some('-') {
                    s.push(self.advance().unwrap());
                }
            } else {
                break;
            }
        }

        let s_clean: String = s.chars().filter(|c| *c != '_').collect();
        if is_float {
            s_clean
                .parse::<f64>()
                .map(Token::Float)
                .map_err(|e| format!("Invalid float '{}': {}", s, e))
        } else {
            s_clean
                .parse::<i64>()
                .map(Token::Integer)
                .map_err(|e| format!("Invalid integer '{}': {}", s, e))
        }
    }

    fn read_symbol(&mut self) -> Result<Token, String> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if is_symbol_char(c) || c == ':' && !s.is_empty() {
                // allow :: in symbols like Type::method
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        // Check for special symbols
        match s.as_str() {
            "true" => Ok(Token::Bool(true)),
            "false" => Ok(Token::Bool(false)),
            "nil" => Ok(Token::Nil),
            _ => Ok(Token::Symbol(s)),
        }
    }
}

fn is_symbol_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_symbol_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '!' | '?' | '+' | '*' | '/' | '<' | '>' | '=' | '%' | '&' | '^' | '|' | '~')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lex = Lexer::new("(defn add [a b] (+ a b))");
        let tokens = lex.tokenize().unwrap();
        assert!(matches!(tokens[0].token, Token::LParen));
        assert!(matches!(&tokens[1].token, Token::Symbol(s) if s == "defn"));
        assert!(matches!(&tokens[2].token, Token::Symbol(s) if s == "add"));
        assert!(matches!(tokens[3].token, Token::LBracket));
    }

    #[test]
    fn test_numbers() {
        let mut lex = Lexer::new("42 3.14 -7");
        let tokens = lex.tokenize().unwrap();
        assert!(matches!(tokens[0].token, Token::Integer(42)));
        assert!(matches!(tokens[1].token, Token::Float(f) if (f - 3.14).abs() < 0.001));
        assert!(matches!(tokens[2].token, Token::Integer(-7)));
    }

    #[test]
    fn test_string_and_keyword() {
        let mut lex = Lexer::new(r#""hello" :world"#);
        let tokens = lex.tokenize().unwrap();
        assert!(matches!(&tokens[0].token, Token::String(s) if s == "hello"));
        assert!(matches!(&tokens[1].token, Token::Keyword(s) if s == "world"));
    }
}
