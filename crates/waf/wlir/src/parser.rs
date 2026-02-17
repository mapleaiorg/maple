//! S-expression parser for WLIR operator bodies and module definitions.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::WlirError;

/// An S-expression node.
///
/// WLIR uses S-expressions as the textual representation for operator
/// bodies.  `SExpr` supports atoms (symbols, numbers, strings) and
/// nested lists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SExpr {
    /// A leaf token (symbol, number literal, string literal, etc.).
    Atom(String),
    /// A parenthesised list of sub-expressions.
    List(Vec<SExpr>),
}

impl fmt::Display for SExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SExpr::Atom(s) => write!(f, "{}", s),
            SExpr::List(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Parse an S-expression string into an `SExpr` tree.
///
/// Accepts inputs like `"foo"`, `"(+ 1 2)"`, or `"(define (f x) (+ x 1))"`.
///
/// # Errors
///
/// Returns [`WlirError::ParseError`] on unbalanced parentheses, empty
/// input, or other syntax issues.
pub fn parse_sexpr(input: &str) -> Result<SExpr, WlirError> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(WlirError::ParseError("empty input".into()));
    }
    let (expr, rest) = parse_tokens(&tokens)?;
    if !rest.is_empty() {
        return Err(WlirError::ParseError(format!(
            "unexpected trailing tokens: {:?}",
            rest
        )));
    }
    Ok(expr)
}

/// Tokenize input into parentheses and atom strings.
fn tokenize(input: &str) -> Result<Vec<String>, WlirError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '(' | ')' => {
                tokens.push(ch.to_string());
                chars.next();
            }
            c if c.is_whitespace() => {
                chars.next();
            }
            _ => {
                let mut atom = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '(' || c == ')' || c.is_whitespace() {
                        break;
                    }
                    atom.push(c);
                    chars.next();
                }
                if atom.is_empty() {
                    return Err(WlirError::ParseError("empty atom".into()));
                }
                tokens.push(atom);
            }
        }
    }

    Ok(tokens)
}

/// Recursively parse a token slice into an SExpr.
fn parse_tokens<'a>(tokens: &'a [String]) -> Result<(SExpr, &'a [String]), WlirError> {
    if tokens.is_empty() {
        return Err(WlirError::ParseError("unexpected end of input".into()));
    }

    if tokens[0] == "(" {
        let mut rest = &tokens[1..];
        let mut items = Vec::new();

        loop {
            if rest.is_empty() {
                return Err(WlirError::ParseError("unbalanced parentheses: missing ')'".into()));
            }
            if rest[0] == ")" {
                rest = &rest[1..];
                break;
            }
            let (item, new_rest) = parse_tokens(rest)?;
            items.push(item);
            rest = new_rest;
        }

        Ok((SExpr::List(items), rest))
    } else if tokens[0] == ")" {
        Err(WlirError::ParseError("unexpected ')'".into()))
    } else {
        Ok((SExpr::Atom(tokens[0].clone()), &tokens[1..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_atom() {
        let expr = parse_sexpr("foo").unwrap();
        assert_eq!(expr, SExpr::Atom("foo".into()));
    }

    #[test]
    fn parse_simple_list() {
        let expr = parse_sexpr("(+ 1 2)").unwrap();
        assert_eq!(
            expr,
            SExpr::List(vec![
                SExpr::Atom("+".into()),
                SExpr::Atom("1".into()),
                SExpr::Atom("2".into()),
            ])
        );
    }

    #[test]
    fn parse_nested_list() {
        let expr = parse_sexpr("(foo bar (baz 42))").unwrap();
        assert_eq!(
            expr,
            SExpr::List(vec![
                SExpr::Atom("foo".into()),
                SExpr::Atom("bar".into()),
                SExpr::List(vec![
                    SExpr::Atom("baz".into()),
                    SExpr::Atom("42".into()),
                ]),
            ])
        );
    }

    #[test]
    fn display_roundtrip() {
        let input = "(define (square x) (* x x))";
        let expr = parse_sexpr(input).unwrap();
        let output = expr.to_string();
        let re_parsed = parse_sexpr(&output).unwrap();
        assert_eq!(expr, re_parsed);
    }

    #[test]
    fn parse_empty_input_error() {
        let result = parse_sexpr("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WlirError::ParseError(_)));
    }

    #[test]
    fn parse_unbalanced_parens_error() {
        let result = parse_sexpr("(foo bar");
        assert!(result.is_err());

        let result2 = parse_sexpr("foo bar)");
        assert!(result2.is_err());
    }
}
