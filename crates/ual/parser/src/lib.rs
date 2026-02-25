//! UAL Parser - Tokenizer and parser for Universal Agent Language.

#![deny(unsafe_code)]

use thiserror::Error;
use ual_types::{CommitStatement, OperationStatement, ReversibilitySpec, UalStatement};

#[derive(Debug, Error)]
pub enum UalParseError {
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("Expected keyword: {0}")]
    ExpectedKeyword(String),
    #[error("Invalid number: {0}")]
    InvalidNumber(String),
    #[error("Parse error: {0}")]
    Message(String),
}

#[derive(Debug, Clone)]
enum Token {
    Ident(String),
    Str(String),
    Number(String),
    Symbol(char),
}

pub fn parse(input: &str) -> Result<Vec<UalStatement>, UalParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_all()
}

fn tokenize(input: &str) -> Result<Vec<Token>, UalParseError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '-' {
            chars.next();
            if let Some('-') = chars.peek().copied() {
                chars.next();
                while let Some(c) = chars.next() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            } else {
                tokens.push(Token::Ident("-".to_string()));
                continue;
            }
        }

        if ch == '\'' || ch == '"' {
            let quote = ch;
            chars.next();
            let mut value = String::new();
            while let Some(c) = chars.next() {
                if c == quote {
                    break;
                }
                if c == '\\' {
                    if let Some(escaped) = chars.next() {
                        value.push(escaped);
                        continue;
                    }
                    return Err(UalParseError::UnexpectedEof);
                }
                value.push(c);
            }
            tokens.push(Token::Str(value));
            continue;
        }

        if ch.is_ascii_digit() {
            let mut value = String::new();
            while let Some(c) = chars.peek().copied() {
                if c.is_ascii_digit() {
                    value.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token::Number(value));
            continue;
        }

        if is_ident_start(ch) {
            let mut value = String::new();
            while let Some(c) = chars.peek().copied() {
                if is_ident_char(c) {
                    value.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token::Ident(value));
            continue;
        }

        if matches!(ch, ';' | ',' | '(' | ')' | '=') {
            tokens.push(Token::Symbol(ch));
            chars.next();
            continue;
        }

        return Err(UalParseError::UnexpectedToken(ch.to_string()));
    }

    Ok(tokens)
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '$')
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_all(&mut self) -> Result<Vec<UalStatement>, UalParseError> {
        let mut statements = Vec::new();
        while self.skip_separators() {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }
        Ok(statements)
    }

    fn skip_separators(&mut self) -> bool {
        while let Some(Token::Symbol(';')) = self.peek() {
            self.pos += 1;
        }
        !self.eof()
    }

    fn parse_statement(&mut self) -> Result<UalStatement, UalParseError> {
        let kw = self.consume_ident_upper()?;
        match kw.as_str() {
            "COMMIT" => self.parse_commit().map(UalStatement::Commit),
            "CREATE" => self.parse_create().map(UalStatement::Operation),
            "UPDATE" => self.parse_update().map(UalStatement::Operation),
            "DEPRECATE" => self.parse_deprecate().map(UalStatement::Operation),
            "SCALE" => self.parse_scale().map(UalStatement::Operation),
            "DELETE" => self.parse_delete().map(UalStatement::Operation),
            "ROLLBACK" => self.parse_rollback().map(UalStatement::Operation),
            "PAUSE" => self.parse_pause().map(UalStatement::Operation),
            "RESUME" => self.parse_resume().map(UalStatement::Operation),
            "RESTART" => self.parse_restart().map(UalStatement::Operation),
            "TERMINATE" => self.parse_terminate().map(UalStatement::Operation),
            "MIGRATE" => self.parse_migrate().map(UalStatement::Operation),
            "DRAIN" => self.parse_drain().map(UalStatement::Operation),
            "CHECKPOINT" => self.parse_checkpoint().map(UalStatement::Operation),
            "RESTORE" => self.parse_restore().map(UalStatement::Operation),
            "HEALTH" => self.parse_health().map(UalStatement::Operation),
            "FORCE" => self.parse_force().map(UalStatement::Operation),
            "CONFIGURE" => self.parse_configure().map(UalStatement::Operation),
            "VIEW" => self.parse_view().map(UalStatement::Operation),
            other => Err(UalParseError::UnexpectedToken(other.to_string())),
        }
    }

    fn parse_commit(&mut self) -> Result<CommitStatement, UalParseError> {
        self.consume_keyword("BY")?;
        let principal = self.consume_value()?;
        self.consume_keyword("DOMAIN")?;
        let domain = self.consume_value()?;
        self.consume_keyword("OUTCOME")?;
        let outcome = self.consume_value()?;

        let mut stmt = CommitStatement {
            principal,
            domain,
            outcome,
            scope: None,
            targets: Vec::new(),
            tags: Vec::new(),
            reversibility: None,
            valid_from: None,
            valid_until: None,
        };

        loop {
            if self.eof() {
                break;
            }
            if let Some(Token::Symbol(';')) = self.peek() {
                break;
            }
            let kw = self.consume_ident_upper()?;
            match kw.as_str() {
                "SCOPE" => {
                    stmt.scope = Some(self.consume_value()?);
                }
                "TARGET" => {
                    stmt.targets.push(self.consume_value()?);
                }
                "TAG" => {
                    stmt.tags.push(self.consume_value()?);
                }
                "REVERSIBLE" => {
                    stmt.reversibility = Some(ReversibilitySpec::Reversible);
                }
                "IRREVERSIBLE" => {
                    stmt.reversibility = Some(ReversibilitySpec::Irreversible);
                }
                "VALID_FROM" => {
                    stmt.valid_from = Some(self.consume_value()?);
                }
                "VALID_UNTIL" => {
                    stmt.valid_until = Some(self.consume_value()?);
                }
                "VALID" => {
                    let next = self.consume_ident_upper()?;
                    match next.as_str() {
                        "FROM" => stmt.valid_from = Some(self.consume_value()?),
                        "UNTIL" => stmt.valid_until = Some(self.consume_value()?),
                        _ => {
                            return Err(UalParseError::ExpectedKeyword("FROM or UNTIL".to_string()))
                        }
                    }
                }
                other => return Err(UalParseError::UnexpectedToken(other.to_string())),
            }
        }

        Ok(stmt)
    }

    fn parse_create(&mut self) -> Result<OperationStatement, UalParseError> {
        let target = self.consume_ident_upper()?;
        match target.as_str() {
            "SPEC" => {
                let spec_id = self.consume_value()?;
                let version = if self.peek_is_keyword("VERSION") {
                    self.consume_keyword("VERSION")?;
                    Some(self.consume_value()?)
                } else {
                    None
                };
                Ok(OperationStatement::CreateSpec { spec_id, version })
            }
            "DEPLOYMENT" => {
                self.consume_keyword("SPEC")?;
                let spec_id = self.consume_value()?;
                let mut replicas = 1;
                if self.peek_is_keyword("REPLICAS") {
                    self.consume_keyword("REPLICAS")?;
                    replicas = self.consume_u32()?;
                }
                Ok(OperationStatement::CreateDeployment { spec_id, replicas })
            }
            other => Err(UalParseError::UnexpectedToken(other.to_string())),
        }
    }

    fn parse_update(&mut self) -> Result<OperationStatement, UalParseError> {
        let target = self.consume_ident_upper()?;
        match target.as_str() {
            "SPEC" => {
                let spec_id = self.consume_value()?;
                let version = if self.peek_is_keyword("VERSION") {
                    self.consume_keyword("VERSION")?;
                    Some(self.consume_value()?)
                } else {
                    None
                };
                Ok(OperationStatement::UpdateSpec { spec_id, version })
            }
            "DEPLOYMENT" => {
                let deployment_id = self.consume_value()?;
                Ok(OperationStatement::UpdateDeployment { deployment_id })
            }
            other => Err(UalParseError::UnexpectedToken(other.to_string())),
        }
    }

    fn parse_deprecate(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("SPEC")?;
        let spec_id = self.consume_value()?;
        Ok(OperationStatement::DeprecateSpec { spec_id })
    }

    fn parse_scale(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("DEPLOYMENT")?;
        let deployment_id = self.consume_value()?;
        let next = self.consume_ident_upper()?;
        let target_replicas = match next.as_str() {
            "TO" | "REPLICAS" => self.consume_u32()?,
            _ => return Err(UalParseError::ExpectedKeyword("TO".to_string())),
        };
        Ok(OperationStatement::ScaleDeployment {
            deployment_id,
            target_replicas,
        })
    }

    fn parse_delete(&mut self) -> Result<OperationStatement, UalParseError> {
        let target = self.consume_ident_upper()?;
        match target.as_str() {
            "DEPLOYMENT" => {
                let deployment_id = self.consume_value()?;
                Ok(OperationStatement::DeleteDeployment { deployment_id })
            }
            "CHECKPOINT" | "SNAPSHOT" => {
                let snapshot_id = self.consume_value()?;
                Ok(OperationStatement::DeleteCheckpoint { snapshot_id })
            }
            other => Err(UalParseError::UnexpectedToken(other.to_string())),
        }
    }

    fn parse_rollback(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("DEPLOYMENT")?;
        let deployment_id = self.consume_value()?;
        Ok(OperationStatement::RollbackDeployment { deployment_id })
    }

    fn parse_pause(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("DEPLOYMENT")?;
        let deployment_id = self.consume_value()?;
        Ok(OperationStatement::PauseDeployment { deployment_id })
    }

    fn parse_resume(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("DEPLOYMENT")?;
        let deployment_id = self.consume_value()?;
        Ok(OperationStatement::ResumeDeployment { deployment_id })
    }

    fn parse_restart(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("INSTANCE")?;
        let instance_id = self.consume_value()?;
        Ok(OperationStatement::RestartInstance { instance_id })
    }

    fn parse_terminate(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("INSTANCE")?;
        let instance_id = self.consume_value()?;
        Ok(OperationStatement::TerminateInstance { instance_id })
    }

    fn parse_migrate(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("INSTANCE")?;
        let instance_id = self.consume_value()?;
        Ok(OperationStatement::MigrateInstance { instance_id })
    }

    fn parse_drain(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("INSTANCE")?;
        let instance_id = self.consume_value()?;
        Ok(OperationStatement::DrainInstance { instance_id })
    }

    fn parse_checkpoint(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("INSTANCE")?;
        let instance_id = self.consume_value()?;
        Ok(OperationStatement::CreateCheckpoint { instance_id })
    }

    fn parse_restore(&mut self) -> Result<OperationStatement, UalParseError> {
        let target = self.consume_ident_upper()?;
        match target.as_str() {
            "CHECKPOINT" | "INSTANCE" => {
                let instance_id = self.consume_value()?;
                Ok(OperationStatement::RestoreCheckpoint { instance_id })
            }
            other => Err(UalParseError::UnexpectedToken(other.to_string())),
        }
    }

    fn parse_health(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("CHECK")?;
        self.consume_keyword("INSTANCE")?;
        let instance_id = self.consume_value()?;
        Ok(OperationStatement::HealthCheck { instance_id })
    }

    fn parse_force(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("RECOVERY")?;
        self.consume_keyword("INSTANCE")?;
        let instance_id = self.consume_value()?;
        Ok(OperationStatement::ForceRecovery { instance_id })
    }

    fn parse_configure(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("POLICY")?;
        let policy_name = self.consume_value()?;
        Ok(OperationStatement::ConfigurePolicy { policy_name })
    }

    fn parse_view(&mut self) -> Result<OperationStatement, UalParseError> {
        self.consume_keyword("AUDIT")?;
        self.consume_keyword("LOG")?;
        let filter = self.consume_value()?;
        Ok(OperationStatement::ViewAuditLog { filter })
    }

    fn consume_ident_upper(&mut self) -> Result<String, UalParseError> {
        match self.next() {
            Some(Token::Ident(value)) => Ok(value.to_uppercase()),
            Some(token) => Err(UalParseError::UnexpectedToken(format!("{:?}", token))),
            None => Err(UalParseError::UnexpectedEof),
        }
    }

    fn consume_keyword(&mut self, keyword: &str) -> Result<(), UalParseError> {
        let value = self.consume_ident_upper()?;
        if value == keyword {
            Ok(())
        } else {
            Err(UalParseError::ExpectedKeyword(keyword.to_string()))
        }
    }

    fn consume_value(&mut self) -> Result<String, UalParseError> {
        match self.next() {
            Some(Token::Ident(value)) => Ok(value),
            Some(Token::Str(value)) => Ok(value),
            Some(Token::Number(value)) => Ok(value),
            Some(token) => Err(UalParseError::UnexpectedToken(format!("{:?}", token))),
            None => Err(UalParseError::UnexpectedEof),
        }
    }

    fn consume_u32(&mut self) -> Result<u32, UalParseError> {
        let value = self.consume_value()?;
        value
            .parse::<u32>()
            .map_err(|_| UalParseError::InvalidNumber(value))
    }

    fn peek_is_keyword(&self, keyword: &str) -> bool {
        match self.peek() {
            Some(Token::Ident(value)) => value.eq_ignore_ascii_case(keyword),
            _ => false,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos >= self.tokens.len() {
            return None;
        }
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        Some(token)
    }

    fn eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }
}
