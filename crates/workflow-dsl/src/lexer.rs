//! Lexer: tokenizes the workflow DSL input
//!
//! Produces a stream of tokens that the parser consumes.
//! Handles keywords, identifiers, string literals, numbers,
//! and structural tokens ({, }, ->, etc.).

use crate::errors::{DslError, DslResult};

/// A token produced by the lexer
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// The raw text of the token
    pub text: String,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub col: usize,
}

impl Token {
    pub fn new(kind: TokenKind, text: impl Into<String>, line: usize, col: usize) -> Self {
        Self {
            kind,
            text: text.into(),
            line,
            col,
        }
    }
}

/// Token types
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    Workflow,
    Version,
    Timeout,
    Roles,
    Node,
    Type,
    Commitment,
    Receipt,
    Escalation,
    Edges,
    Role,
    On,

    // Identifiers and literals
    Identifier,
    StringLiteral,
    NumberLiteral,

    // Structural
    OpenBrace,
    CloseBrace,
    Arrow, // ->
    Colon,

    // End of input
    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Workflow => write!(f, "WORKFLOW"),
            Self::Version => write!(f, "VERSION"),
            Self::Timeout => write!(f, "TIMEOUT"),
            Self::Roles => write!(f, "ROLES"),
            Self::Node => write!(f, "NODE"),
            Self::Type => write!(f, "TYPE"),
            Self::Commitment => write!(f, "COMMITMENT"),
            Self::Receipt => write!(f, "RECEIPT"),
            Self::Escalation => write!(f, "ESCALATION"),
            Self::Edges => write!(f, "EDGES"),
            Self::Role => write!(f, "ROLE"),
            Self::On => write!(f, "ON"),
            Self::Identifier => write!(f, "identifier"),
            Self::StringLiteral => write!(f, "string literal"),
            Self::NumberLiteral => write!(f, "number"),
            Self::OpenBrace => write!(f, "{{"),
            Self::CloseBrace => write!(f, "}}"),
            Self::Arrow => write!(f, "->"),
            Self::Colon => write!(f, ":"),
            Self::Eof => write!(f, "end of input"),
        }
    }
}

/// Lexer for the workflow DSL
pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    /// Create a new lexer from input text
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> DslResult<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.pos >= self.input.len() {
                tokens.push(Token::new(TokenKind::Eof, "", self.line, self.col));
                break;
            }

            let token = self.next_token()?;
            tokens.push(token);
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> DslResult<Token> {
        let ch = self.input[self.pos];
        let line = self.line;
        let col = self.col;

        match ch {
            '{' => {
                self.advance();
                Ok(Token::new(TokenKind::OpenBrace, "{", line, col))
            }
            '}' => {
                self.advance();
                Ok(Token::new(TokenKind::CloseBrace, "}", line, col))
            }
            ':' => {
                self.advance();
                Ok(Token::new(TokenKind::Colon, ":", line, col))
            }
            '-' if self.peek_at(1) == Some('>') => {
                self.advance();
                self.advance();
                Ok(Token::new(TokenKind::Arrow, "->", line, col))
            }
            '"' => self.read_string_literal(),
            c if c.is_ascii_digit() => self.read_number(),
            c if c.is_ascii_alphabetic() || c == '_' => self.read_identifier_or_keyword(),
            _ => Err(DslError::ParseError {
                line,
                col,
                message: format!("Unexpected character: '{}'", ch),
            }),
        }
    }

    fn read_string_literal(&mut self) -> DslResult<Token> {
        let line = self.line;
        let col = self.col;
        self.advance(); // skip opening quote

        let mut text = String::new();
        while self.pos < self.input.len() && self.input[self.pos] != '"' {
            if self.input[self.pos] == '\\' && self.peek_at(1) == Some('"') {
                self.advance();
                text.push('"');
            } else {
                text.push(self.input[self.pos]);
            }
            self.advance();
        }

        if self.pos >= self.input.len() {
            return Err(DslError::ParseError {
                line,
                col,
                message: "Unterminated string literal".into(),
            });
        }

        self.advance(); // skip closing quote
        Ok(Token::new(TokenKind::StringLiteral, text, line, col))
    }

    fn read_number(&mut self) -> DslResult<Token> {
        let line = self.line;
        let col = self.col;
        let mut text = String::new();

        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            text.push(self.input[self.pos]);
            self.advance();
        }

        Ok(Token::new(TokenKind::NumberLiteral, text, line, col))
    }

    fn read_identifier_or_keyword(&mut self) -> DslResult<Token> {
        let line = self.line;
        let col = self.col;
        let mut text = String::new();

        while self.pos < self.input.len()
            && (self.input[self.pos].is_ascii_alphanumeric() || self.input[self.pos] == '_')
        {
            text.push(self.input[self.pos]);
            self.advance();
        }

        let kind = match text.as_str() {
            "WORKFLOW" => TokenKind::Workflow,
            "VERSION" => TokenKind::Version,
            "TIMEOUT" => TokenKind::Timeout,
            "ROLES" => TokenKind::Roles,
            "NODE" => TokenKind::Node,
            "TYPE" => TokenKind::Type,
            "COMMITMENT" => TokenKind::Commitment,
            "RECEIPT" => TokenKind::Receipt,
            "ESCALATION" => TokenKind::Escalation,
            "EDGES" => TokenKind::Edges,
            "ROLE" => TokenKind::Role,
            "ON" => TokenKind::On,
            _ => TokenKind::Identifier,
        };

        Ok(Token::new(kind, text, line, col))
    }

    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '#' || (ch == '/' && self.peek_at(1) == Some('/')) {
                // Line comment
                while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn advance(&mut self) {
        if self.pos < self.input.len() {
            if self.input[self.pos] == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.input.get(self.pos + offset).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("WORKFLOW \"test\" { }");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Workflow);
        assert_eq!(tokens[1].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[1].text, "test");
        assert_eq!(tokens[2].kind, TokenKind::OpenBrace);
        assert_eq!(tokens[3].kind, TokenKind::CloseBrace);
        assert_eq!(tokens[4].kind, TokenKind::Eof);
    }

    #[test]
    fn test_arrow_token() {
        let mut lexer = Lexer::new("start -> end");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].text, "start");
        assert_eq!(tokens[1].kind, TokenKind::Arrow);
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
        assert_eq!(tokens[2].text, "end");
    }

    #[test]
    fn test_number_literal() {
        let mut lexer = Lexer::new("TIMEOUT 3600");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Timeout);
        assert_eq!(tokens[1].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[1].text, "3600");
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new(
            "WORKFLOW NODE TYPE EDGES ROLES COMMITMENT RECEIPT ESCALATION ON ROLE VERSION TIMEOUT",
        );
        let tokens = lexer.tokenize().unwrap();

        let expected = vec![
            TokenKind::Workflow,
            TokenKind::Node,
            TokenKind::Type,
            TokenKind::Edges,
            TokenKind::Roles,
            TokenKind::Commitment,
            TokenKind::Receipt,
            TokenKind::Escalation,
            TokenKind::On,
            TokenKind::Role,
            TokenKind::Version,
            TokenKind::Timeout,
            TokenKind::Eof,
        ];

        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(tokens[i].kind, *exp, "Token {} mismatch", i);
        }
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("WORKFLOW # this is a comment\n\"test\"");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Workflow);
        assert_eq!(tokens[1].kind, TokenKind::StringLiteral);
    }

    #[test]
    fn test_line_tracking() {
        let mut lexer = Lexer::new("WORKFLOW\n\"test\"\n{}");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[1].line, 2);
        assert_eq!(tokens[2].line, 3);
    }

    #[test]
    fn test_unterminated_string() {
        let mut lexer = Lexer::new("\"unterminated");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_colon_token() {
        let mut lexer = Lexer::new("reviewer: \"description\"");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].kind, TokenKind::Colon);
        assert_eq!(tokens[2].kind, TokenKind::StringLiteral);
    }

    #[test]
    fn test_identifier_with_underscores() {
        let mut lexer = Lexer::new("my_node some_role");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].text, "my_node");
        assert_eq!(tokens[1].text, "some_role");
    }

    #[test]
    fn test_empty_input() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_slash_slash_comments() {
        let mut lexer = Lexer::new("WORKFLOW // comment\n\"test\"");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Workflow);
        assert_eq!(tokens[1].kind, TokenKind::StringLiteral);
    }
}
