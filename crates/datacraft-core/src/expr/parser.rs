//! Expression parser (step 15.1, item H41).
//!
//! Recursive descent over the documented precedence ladder (see the
//! module docs of [`super`]). `Between … And` and `In (…)` bind at the
//! comparison level, so their inner operands parse at concatenation level
//! and never swallow the surrounding `And`/`Or`.

use super::lexer::{Spanned, Token, lex};
use super::{BinaryOp, CompareOp, ExprError, Expression, UnaryOp};
use datacraft_types::parse_date;

/// Parses the source into an AST; the whole token stream must be consumed.
pub(crate) fn parse(source: &str) -> Result<Expression, ExprError> {
    let tokens = lex(source)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        end: source.len(),
    };
    if parser.peek().is_none() {
        return Err(parser.error("empty expression"));
    }
    let expression = parser.or_expr()?;
    if let Some(spanned) = parser.peek_spanned() {
        return Err(ExprError::Parse {
            position: spanned.start,
            detail: "unexpected token after the expression".to_owned(),
        });
    }
    Ok(expression)
}

struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
    end: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|spanned| &spanned.token)
    }

    fn peek_spanned(&self) -> Option<&Spanned> {
        self.tokens.get(self.pos)
    }

    fn eat(&mut self, token: &Token) -> bool {
        if self.peek() == Some(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &Token, detail: &str) -> Result<(), ExprError> {
        if self.eat(token) {
            Ok(())
        } else {
            Err(self.error(detail))
        }
    }

    fn error(&self, detail: impl Into<String>) -> ExprError {
        ExprError::Parse {
            position: self
                .peek_spanned()
                .map(|spanned| spanned.start)
                .unwrap_or(self.end),
            detail: detail.into(),
        }
    }

    /// `Or` — lowest precedence, left-associative.
    fn or_expr(&mut self) -> Result<Expression, ExprError> {
        let mut left = self.and_expr()?;
        while self.eat(&Token::Or) {
            let right = self.and_expr()?;
            left = Expression::Binary(BinaryOp::Or, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<Expression, ExprError> {
        let mut left = self.not_expr()?;
        while self.eat(&Token::And) {
            let right = self.not_expr()?;
            left = Expression::Binary(BinaryOp::And, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// Logical `Not` binds below the comparisons (`Not 2 < 3` is `Not (2<3)`).
    fn not_expr(&mut self) -> Result<Expression, ExprError> {
        if self.eat(&Token::Not) {
            let operand = self.not_expr()?;
            Ok(Expression::Unary(UnaryOp::Not, Box::new(operand)))
        } else {
            self.comparison()
        }
    }

    fn comparison(&mut self) -> Result<Expression, ExprError> {
        let mut left = self.concat_expr()?;
        loop {
            if let Some(op) = self.compare_op() {
                let right = self.concat_expr()?;
                left = Expression::Compare(op, Box::new(left), Box::new(right));
            } else if self.eat(&Token::Between) {
                let low = self.concat_expr()?;
                self.expect(&Token::And, "Between expects 'And'")?;
                let high = self.concat_expr()?;
                left = Expression::Between(Box::new(left), Box::new(low), Box::new(high));
            } else if self.eat(&Token::In) {
                self.expect(&Token::LParen, "In expects '('")?;
                let mut list = vec![self.or_expr()?];
                while self.eat(&Token::Comma) {
                    list.push(self.or_expr()?);
                }
                self.expect(&Token::RParen, "In expects a ')' after the list")?;
                left = Expression::InList(Box::new(left), list);
            } else if self.eat(&Token::Is) {
                let negated = self.eat(&Token::Not);
                self.expect(&Token::NullKw, "Is expects Null")?;
                left = Expression::IsNull {
                    operand: Box::new(left),
                    negated,
                };
            } else {
                return Ok(left);
            }
        }
    }

    fn compare_op(&mut self) -> Option<CompareOp> {
        let op = match self.peek()? {
            Token::Eq => CompareOp::Equal,
            Token::Ne => CompareOp::NotEqual,
            Token::Lt => CompareOp::Less,
            Token::Le => CompareOp::LessEqual,
            Token::Gt => CompareOp::Greater,
            Token::Ge => CompareOp::GreaterEqual,
            Token::Like => CompareOp::Like,
            _ => return None,
        };
        self.pos += 1;
        Some(op)
    }

    fn concat_expr(&mut self) -> Result<Expression, ExprError> {
        let mut left = self.additive()?;
        while self.eat(&Token::Amp) {
            let right = self.additive()?;
            left = Expression::Binary(BinaryOp::Concat, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn additive(&mut self) -> Result<Expression, ExprError> {
        let mut left = self.modulo_expr()?;
        loop {
            let op = if self.eat(&Token::Plus) {
                BinaryOp::Add
            } else if self.eat(&Token::Minus) {
                BinaryOp::Subtract
            } else {
                return Ok(left);
            };
            let right = self.modulo_expr()?;
            left = Expression::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn modulo_expr(&mut self) -> Result<Expression, ExprError> {
        let mut left = self.integer_divide()?;
        while self.eat(&Token::Mod) {
            let right = self.integer_divide()?;
            left = Expression::Binary(BinaryOp::Modulo, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn integer_divide(&mut self) -> Result<Expression, ExprError> {
        let mut left = self.term()?;
        while self.eat(&Token::Backslash) {
            let right = self.term()?;
            left = Expression::Binary(BinaryOp::IntegerDivide, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expression, ExprError> {
        let mut left = self.unary()?;
        loop {
            let op = if self.eat(&Token::Star) {
                BinaryOp::Multiply
            } else if self.eat(&Token::Slash) {
                BinaryOp::Divide
            } else {
                return Ok(left);
            };
            let right = self.unary()?;
            left = Expression::Binary(op, Box::new(left), Box::new(right));
        }
    }

    /// `^` is right-associative and binds tighter than unary minus
    /// (`-2^2` is `-(2^2)`; `2^-2` works through the unary recursion).
    fn power(&mut self) -> Result<Expression, ExprError> {
        let base = self.primary()?;
        if self.eat(&Token::Caret) {
            let exponent = self.unary()?;
            Ok(Expression::Binary(
                BinaryOp::Power,
                Box::new(base),
                Box::new(exponent),
            ))
        } else {
            Ok(base)
        }
    }

    fn unary(&mut self) -> Result<Expression, ExprError> {
        if self.eat(&Token::Minus) {
            let operand = self.unary()?;
            Ok(Expression::Unary(UnaryOp::Negate, Box::new(operand)))
        } else {
            self.power()
        }
    }

    fn primary(&mut self) -> Result<Expression, ExprError> {
        let Some(spanned) = self.peek_spanned().cloned() else {
            return Err(self.error("unexpected end of expression"));
        };
        match spanned.token {
            Token::Integer(value) => {
                self.pos += 1;
                Ok(Expression::Literal(crate::schema::FieldValue::Integer(
                    value,
                )))
            }
            Token::Float(value) => {
                self.pos += 1;
                Ok(Expression::Literal(crate::schema::FieldValue::Float(value)))
            }
            Token::Text(text) => {
                self.pos += 1;
                Ok(Expression::Literal(crate::schema::FieldValue::Text(text)))
            }
            Token::Date(text) => {
                self.pos += 1;
                match parse_date(&text) {
                    Some(value) => Ok(Expression::Literal(value)),
                    None => Err(ExprError::Parse {
                        position: spanned.start,
                        detail: format!("{text:?} is not a valid date literal"),
                    }),
                }
            }
            Token::True => {
                self.pos += 1;
                Ok(Expression::Literal(crate::schema::FieldValue::Boolean(
                    true,
                )))
            }
            Token::False => {
                self.pos += 1;
                Ok(Expression::Literal(crate::schema::FieldValue::Boolean(
                    false,
                )))
            }
            Token::NullKw => {
                self.pos += 1;
                Ok(Expression::Literal(crate::schema::FieldValue::Null))
            }
            Token::Ident(name) => {
                self.pos += 1;
                self.ident_tail(Some(name), true)
            }
            Token::Bracket(name) => {
                self.pos += 1;
                self.ident_tail(Some(name), false)
            }
            Token::LParen => {
                self.pos += 1;
                let inner = self.or_expr()?;
                self.expect(&Token::RParen, "expected ')'")?;
                Ok(inner)
            }
            _ => Err(self.error("unexpected token")),
        }
    }

    /// After a leading name: a call `Name(…)`, a qualified reference
    /// `Name.rest` or a plain field reference. Bracketed names can never
    /// start a call (Access function names are never bracketed).
    fn ident_tail(
        &mut self,
        name: Option<String>,
        callable: bool,
    ) -> Result<Expression, ExprError> {
        if callable && self.peek() == Some(&Token::LParen) {
            self.pos += 1;
            let mut args = Vec::new();
            if self.peek() != Some(&Token::RParen) {
                args.push(self.or_expr()?);
                while self.eat(&Token::Comma) {
                    args.push(self.or_expr()?);
                }
            }
            self.expect(&Token::RParen, "expected ')' after the arguments")?;
            return Ok(Expression::Call {
                name: name.expect("checked above"),
                args,
            });
        }
        if self.peek() == Some(&Token::Dot) {
            self.pos += 1;
            let field = match self.peek().cloned() {
                Some(Token::Ident(inner)) => {
                    self.pos += 1;
                    inner
                }
                Some(Token::Bracket(inner)) => {
                    self.pos += 1;
                    inner
                }
                _ => return Err(self.error("expected a field name after '.'")),
            };
            return Ok(Expression::Field {
                table: name,
                name: field,
            });
        }
        Ok(Expression::Field {
            table: None,
            name: name.expect("call path handled Some(name) above"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(source: &str) -> Expression {
        parse(source).unwrap_or_else(|e| panic!("{source:?} must parse: {e}"))
    }

    #[test]
    fn precedence_ladder_groups_as_documented() {
        // * over +
        assert!(matches!(
            parse_ok("2+3*4"),
            Expression::Binary(BinaryOp::Add, _, plus)
                if matches!(*plus, Expression::Binary(BinaryOp::Multiply, _, _))
        ));
        // & sits below + -
        assert!(matches!(
            parse_ok("\"a\" & 1+2"),
            Expression::Binary(BinaryOp::Concat, _, add)
                if matches!(*add, Expression::Binary(BinaryOp::Add, _, _))
        ));
        // Not binds below comparisons
        assert!(matches!(
            parse_ok("Not 2 < 3"),
            Expression::Unary(UnaryOp::Not, compare)
                if matches!(*compare, Expression::Compare(_, _, _))
        ));
        // ^ is right-associative
        assert!(matches!(
            parse_ok("2^3^2"),
            Expression::Binary(BinaryOp::Power, two, nine)
                if matches!(*two, Expression::Literal(_))
                    && matches!(*nine, Expression::Binary(BinaryOp::Power, _, _))
        ));
    }

    #[test]
    fn qualified_references_accept_both_spellings() {
        for source in [
            "Customers.Qty",
            "[Customers].Qty",
            "Customers.[Qty]",
            "[Customers].[Qty]",
        ] {
            assert!(matches!(
                parse_ok(source),
                Expression::Field { table: Some(table), .. } if table == "Customers"
            ));
        }
    }

    #[test]
    fn between_and_in_bind_at_comparison_level() {
        assert!(matches!(
            parse_ok("x Between 1 And 2 Or x In (3, 4)"),
            Expression::Binary(BinaryOp::Or, _, _)
        ));
    }

    #[test]
    fn malformed_sources_report_a_position() {
        for source in ["", "1 +", "(1", "1 2", "1 @ 2", "[T.", "Foo(", "Is Not"] {
            let error = parse(source).expect_err(source);
            assert!(
                matches!(error, ExprError::Parse { .. } | ExprError::Lex { .. }),
                "{source:?}: {error:?}"
            );
        }
    }

    #[test]
    fn date_literals_validate_their_content() {
        assert!(matches!(
            parse_ok("#2025-01-05#"),
            Expression::Literal(crate::schema::FieldValue::DateTime(_))
        ));
        let error = parse("#2025-13-40#").expect_err("invalid date must fail");
        assert!(error.to_string().contains("date literal"));
    }

    /// A bare `.` expects a field name right after it; bracket names are
    /// accepted the same as identifiers.
    #[test]
    fn dotted_field_access_takes_ident_or_bracket_names() {
        assert!(matches!(
            parse_ok("[Customers].[name]"),
            Expression::Field { .. }
        ));
        let error = parse("[Customers].").expect_err("bare dot must fail");
        assert!(error.to_string().contains("field name"), "{error}");
    }

    /// Multi-argument calls parse every argument through the same
    /// expression grammar (instr takes two to three arguments).
    #[test]
    fn calls_parse_every_comma_separated_argument() {
        assert!(matches!(
            parse_ok("instr(1, \"abc\", 2)"),
            Expression::Call { name, .. } if name == "instr"
        ));
    }
}
