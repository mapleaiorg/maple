//! Parser: recursive descent parser for the workflow DSL
//!
//! Consumes tokens from the lexer and produces an intermediate
//! representation (ParsedWorkflow) that the compiler converts
//! into a WorkflowDefinition.

use crate::errors::{DslError, DslResult};
use crate::lexer::{Lexer, Token, TokenKind};

/// Parsed workflow — the intermediate representation
#[derive(Clone, Debug)]
pub struct ParsedWorkflow {
    /// Workflow name
    pub name: String,
    /// Version string
    pub version: Option<String>,
    /// Global timeout in seconds
    pub timeout: Option<u64>,
    /// Declared roles
    pub roles: Vec<ParsedRole>,
    /// Declared nodes
    pub nodes: Vec<ParsedNode>,
    /// Declared edges
    pub edges: Vec<ParsedEdge>,
    /// Escalation rules
    pub escalations: Vec<ParsedEscalation>,
}

/// A parsed role declaration
#[derive(Clone, Debug)]
pub struct ParsedRole {
    pub id: String,
    pub description: String,
}

/// A parsed node declaration
#[derive(Clone, Debug)]
pub struct ParsedNode {
    pub id: String,
    pub node_type: String,
    pub role: Option<String>,
    pub commitment: Option<String>,
    pub receipt: Option<String>,
    pub timeout: Option<u64>,
    pub escalation_action: Option<String>,
    pub escalation_param: Option<String>,
}

/// A parsed edge declaration
#[derive(Clone, Debug)]
pub struct ParsedEdge {
    pub from: String,
    pub to: String,
    pub gate_type: Option<String>,
    pub gate_value: Option<String>,
}

/// A parsed escalation rule
#[derive(Clone, Debug)]
pub struct ParsedEscalation {
    pub trigger: String,
    pub action: String,
    pub param: Option<String>,
}

/// Parser for the workflow DSL
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Parse DSL input text into a ParsedWorkflow
    pub fn parse(input: &str) -> DslResult<ParsedWorkflow> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        let mut parser = Self { tokens, pos: 0 };
        parser.parse_workflow()
    }

    fn parse_workflow(&mut self) -> DslResult<ParsedWorkflow> {
        // WORKFLOW "name" {
        self.expect(TokenKind::Workflow)?;
        let name = self.expect(TokenKind::StringLiteral)?.text.clone();
        self.expect(TokenKind::OpenBrace)?;

        let mut workflow = ParsedWorkflow {
            name,
            version: None,
            timeout: None,
            roles: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            escalations: Vec::new(),
        };

        // Parse body until closing brace
        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::Eof) {
            match self.peek_kind() {
                TokenKind::Version => {
                    self.advance();
                    let version = self.expect(TokenKind::StringLiteral)?.text.clone();
                    workflow.version = Some(version);
                }
                TokenKind::Timeout => {
                    self.advance();
                    let timeout = self.expect_number()?;
                    workflow.timeout = Some(timeout);
                }
                TokenKind::Roles => {
                    workflow.roles = self.parse_roles_block()?;
                }
                TokenKind::Node => {
                    let node = self.parse_node()?;
                    workflow.nodes.push(node);
                }
                TokenKind::Edges => {
                    workflow.edges = self.parse_edges_block()?;
                }
                TokenKind::Escalation => {
                    workflow.escalations = self.parse_escalation_block()?;
                }
                _ => {
                    let tok = self.peek();
                    return Err(DslError::UnknownKeyword(tok.text.clone()));
                }
            }
        }

        self.expect(TokenKind::CloseBrace)?;
        Ok(workflow)
    }

    fn parse_roles_block(&mut self) -> DslResult<Vec<ParsedRole>> {
        self.expect(TokenKind::Roles)?;
        self.expect(TokenKind::OpenBrace)?;

        let mut roles = Vec::new();
        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::Eof) {
            let id = self.expect(TokenKind::Identifier)?.text.clone();
            self.expect(TokenKind::Colon)?;
            let description = self.expect(TokenKind::StringLiteral)?.text.clone();
            roles.push(ParsedRole { id, description });
        }

        self.expect(TokenKind::CloseBrace)?;
        Ok(roles)
    }

    fn parse_node(&mut self) -> DslResult<ParsedNode> {
        self.expect(TokenKind::Node)?;
        let id = self.expect_identifier()?;
        self.expect(TokenKind::Type)?;
        let node_type = self.expect_identifier()?;

        let mut node = ParsedNode {
            id,
            node_type,
            role: None,
            commitment: None,
            receipt: None,
            timeout: None,
            escalation_action: None,
            escalation_param: None,
        };

        // Optional body block
        if self.check(TokenKind::OpenBrace) {
            self.advance();
            while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::Eof) {
                match self.peek_kind() {
                    TokenKind::Role => {
                        self.advance();
                        node.role = Some(self.expect_identifier()?);
                    }
                    TokenKind::Commitment => {
                        self.advance();
                        node.commitment = Some(self.expect(TokenKind::StringLiteral)?.text.clone());
                    }
                    TokenKind::Receipt => {
                        self.advance();
                        node.receipt = Some(self.expect_identifier()?);
                    }
                    TokenKind::Timeout => {
                        self.advance();
                        node.timeout = Some(self.expect_number()?);
                    }
                    TokenKind::Escalation => {
                        self.advance();
                        node.escalation_action = Some(self.expect_identifier()?);
                        if self.check(TokenKind::NumberLiteral) {
                            let val = self.expect_number()?;
                            node.escalation_param = Some(val.to_string());
                        } else if self.check(TokenKind::StringLiteral) {
                            node.escalation_param =
                                Some(self.expect(TokenKind::StringLiteral)?.text.clone());
                        }
                    }
                    _ => {
                        let tok = self.peek();
                        return Err(DslError::ParseError {
                            line: tok.line,
                            col: tok.col,
                            message: format!("Unexpected token in node body: '{}'", tok.text),
                        });
                    }
                }
            }
            self.expect(TokenKind::CloseBrace)?;
        }

        Ok(node)
    }

    fn parse_edges_block(&mut self) -> DslResult<Vec<ParsedEdge>> {
        self.expect(TokenKind::Edges)?;
        self.expect(TokenKind::OpenBrace)?;

        let mut edges = Vec::new();
        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::Eof) {
            let from = self.expect_identifier()?;
            self.expect(TokenKind::Arrow)?;
            let to = self.expect_identifier()?;

            let mut edge = ParsedEdge {
                from,
                to,
                gate_type: None,
                gate_value: None,
            };

            // Optional ON gate
            if self.check(TokenKind::On) {
                self.advance();
                let gate_type = self.expect_identifier()?;
                edge.gate_type = Some(gate_type);

                // Gate value (receipt type, condition, timeout value)
                if self.check(TokenKind::Identifier) {
                    edge.gate_value = Some(self.expect_identifier()?);
                } else if self.check(TokenKind::StringLiteral) {
                    edge.gate_value = Some(self.expect(TokenKind::StringLiteral)?.text.clone());
                } else if self.check(TokenKind::NumberLiteral) {
                    edge.gate_value = Some(self.expect_number()?.to_string());
                }
            }

            edges.push(edge);
        }

        self.expect(TokenKind::CloseBrace)?;
        Ok(edges)
    }

    fn parse_escalation_block(&mut self) -> DslResult<Vec<ParsedEscalation>> {
        self.expect(TokenKind::Escalation)?;
        self.expect(TokenKind::OpenBrace)?;

        let mut escalations = Vec::new();
        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::Eof) {
            self.expect(TokenKind::On)?;
            let trigger = self.expect_identifier()?;
            self.expect(TokenKind::Arrow)?;
            let action = self.expect_identifier()?;

            let mut param = None;
            if self.check(TokenKind::StringLiteral) {
                param = Some(self.expect(TokenKind::StringLiteral)?.text.clone());
            } else if self.check(TokenKind::NumberLiteral) {
                param = Some(self.expect_number()?.to_string());
            } else if self.check(TokenKind::Identifier) {
                param = Some(self.expect_identifier()?);
            }

            escalations.push(ParsedEscalation {
                trigger,
                action,
                param,
            });
        }

        self.expect(TokenKind::CloseBrace)?;
        Ok(escalations)
    }

    // ── Helpers ──────────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_kind(&self) -> TokenKind {
        self.peek().kind.clone()
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: TokenKind) -> DslResult<&Token> {
        if self.check(kind.clone()) {
            Ok(self.advance())
        } else if self.check(TokenKind::Eof) {
            Err(DslError::UnexpectedEof(format!("{}", kind)))
        } else {
            let tok = self.peek();
            Err(DslError::UnexpectedToken {
                expected: format!("{}", kind),
                found: tok.text.clone(),
            })
        }
    }

    fn expect_identifier(&mut self) -> DslResult<String> {
        let tok = self.expect(TokenKind::Identifier)?;
        Ok(tok.text.clone())
    }

    fn expect_number(&mut self) -> DslResult<u64> {
        let tok = self.expect(TokenKind::NumberLiteral)?;
        tok.text.parse::<u64>().map_err(|_| DslError::InvalidValue {
            field: "number".into(),
            message: format!("'{}' is not a valid number", tok.text),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_workflow() {
        let input = r#"
        WORKFLOW "Minimal" {
            NODE start TYPE start
            NODE end TYPE end
            EDGES {
                start -> end
            }
        }
        "#;

        let parsed = Parser::parse(input).unwrap();
        assert_eq!(parsed.name, "Minimal");
        assert_eq!(parsed.nodes.len(), 2);
        assert_eq!(parsed.edges.len(), 1);
    }

    #[test]
    fn test_parse_full_workflow() {
        let input = r#"
        WORKFLOW "Document Review" {
            VERSION "1.0"
            TIMEOUT 86400

            ROLES {
                author: "Document author"
                reviewer: "Document reviewer"
            }

            NODE start TYPE start
            NODE submit TYPE action {
                ROLE author
                COMMITMENT "Submit document"
                TIMEOUT 3600
            }
            NODE review TYPE action {
                ROLE reviewer
                COMMITMENT "Review document"
                RECEIPT CommitmentFulfilled
                TIMEOUT 7200
                ESCALATION timeout_retry 3
            }
            NODE end TYPE end

            EDGES {
                start -> submit
                submit -> review ON receipt CommitmentFulfilled
                review -> end ON receipt CommitmentFulfilled
            }

            ESCALATION {
                ON timeout -> abort "Review timed out"
            }
        }
        "#;

        let parsed = Parser::parse(input).unwrap();
        assert_eq!(parsed.name, "Document Review");
        assert_eq!(parsed.version, Some("1.0".to_string()));
        assert_eq!(parsed.timeout, Some(86400));
        assert_eq!(parsed.roles.len(), 2);
        assert_eq!(parsed.nodes.len(), 4);
        assert_eq!(parsed.edges.len(), 3);
        assert_eq!(parsed.escalations.len(), 1);

        // Check submit node
        let submit = &parsed.nodes[1];
        assert_eq!(submit.id, "submit");
        assert_eq!(submit.node_type, "action");
        assert_eq!(submit.role, Some("author".to_string()));
        assert_eq!(submit.commitment, Some("Submit document".to_string()));
        assert_eq!(submit.timeout, Some(3600));

        // Check review node
        let review = &parsed.nodes[2];
        assert_eq!(review.escalation_action, Some("timeout_retry".to_string()));
        assert_eq!(review.escalation_param, Some("3".to_string()));

        // Check edges
        assert_eq!(parsed.edges[0].from, "start");
        assert_eq!(parsed.edges[0].to, "submit");
        assert!(parsed.edges[0].gate_type.is_none());

        assert_eq!(parsed.edges[1].gate_type, Some("receipt".to_string()));
        assert_eq!(
            parsed.edges[1].gate_value,
            Some("CommitmentFulfilled".to_string())
        );

        // Check escalation
        assert_eq!(parsed.escalations[0].trigger, "timeout");
        assert_eq!(parsed.escalations[0].action, "abort");
        assert_eq!(
            parsed.escalations[0].param,
            Some("Review timed out".to_string())
        );
    }

    #[test]
    fn test_parse_with_comments() {
        let input = r#"
        # This is a workflow
        WORKFLOW "Commented" {
            // Start node
            NODE start TYPE start
            NODE end TYPE end
            EDGES {
                start -> end # automatic
            }
        }
        "#;

        let parsed = Parser::parse(input).unwrap();
        assert_eq!(parsed.name, "Commented");
        assert_eq!(parsed.nodes.len(), 2);
    }

    #[test]
    fn test_parse_decision_workflow() {
        let input = r#"
        WORKFLOW "Decision" {
            NODE start TYPE start
            NODE check TYPE action {
                COMMITMENT "Evaluate request"
            }
            NODE approve TYPE action {
                COMMITMENT "Approve request"
            }
            NODE reject TYPE action {
                COMMITMENT "Reject request"
            }
            NODE end_ok TYPE end
            NODE end_fail TYPE end

            EDGES {
                start -> check
                check -> approve ON condition "approved"
                check -> reject ON condition "rejected"
                approve -> end_ok
                reject -> end_fail
            }
        }
        "#;

        let parsed = Parser::parse(input).unwrap();
        assert_eq!(parsed.nodes.len(), 6);
        assert_eq!(parsed.edges.len(), 5);

        let cond_edge = &parsed.edges[1];
        assert_eq!(cond_edge.gate_type, Some("condition".to_string()));
        assert_eq!(cond_edge.gate_value, Some("approved".to_string()));
    }

    #[test]
    fn test_parse_parallel_workflow() {
        let input = r#"
        WORKFLOW "Parallel" {
            NODE start TYPE start
            NODE fork TYPE fork
            NODE task_a TYPE action { COMMITMENT "Task A" }
            NODE task_b TYPE action { COMMITMENT "Task B" }
            NODE join TYPE join
            NODE end TYPE end

            EDGES {
                start -> fork
                fork -> task_a
                fork -> task_b
                task_a -> join
                task_b -> join
                join -> end
            }
        }
        "#;

        let parsed = Parser::parse(input).unwrap();
        assert_eq!(parsed.nodes.len(), 6);
        assert_eq!(parsed.edges.len(), 6);

        let fork_node = parsed.nodes.iter().find(|n| n.id == "fork").unwrap();
        assert_eq!(fork_node.node_type, "fork");

        let join_node = parsed.nodes.iter().find(|n| n.id == "join").unwrap();
        assert_eq!(join_node.node_type, "join");
    }

    #[test]
    fn test_parse_timeout_edge() {
        let input = r#"
        WORKFLOW "Timeout" {
            NODE start TYPE start
            NODE wait TYPE action { COMMITMENT "Wait" }
            NODE escalate TYPE action { COMMITMENT "Escalate" }
            NODE end TYPE end

            EDGES {
                start -> wait
                wait -> end ON receipt CommitmentFulfilled
                wait -> escalate ON timeout 3600
                escalate -> end
            }
        }
        "#;

        let parsed = Parser::parse(input).unwrap();
        let timeout_edge = &parsed.edges[2];
        assert_eq!(timeout_edge.gate_type, Some("timeout".to_string()));
        assert_eq!(timeout_edge.gate_value, Some("3600".to_string()));
    }

    #[test]
    fn test_parse_error_missing_name() {
        let input = "WORKFLOW {";
        let result = Parser::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_brace() {
        let input = r#"WORKFLOW "Test""#;
        let result = Parser::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_roles() {
        let input = r#"
        WORKFLOW "Roles" {
            ROLES {
                admin: "Administrator"
                user: "Regular user"
                auditor: "Audits operations"
            }
            NODE start TYPE start
            NODE end TYPE end
            EDGES {
                start -> end
            }
        }
        "#;

        let parsed = Parser::parse(input).unwrap();
        assert_eq!(parsed.roles.len(), 3);
        assert_eq!(parsed.roles[0].id, "admin");
        assert_eq!(parsed.roles[0].description, "Administrator");
    }
}
