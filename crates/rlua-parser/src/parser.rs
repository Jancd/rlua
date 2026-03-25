use crate::ParseError;
use crate::ast::*;
use crate::lexer::Lexer;
use crate::token::TokenKind;

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    current: TokenKind,
    line: u32,
    column: u32,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(source);
        let tok = lexer.next_token()?;
        Ok(Self {
            line: tok.span.line,
            column: tok.span.column,
            current: tok.kind,
            lexer,
        })
    }

    fn advance(&mut self) -> Result<TokenKind, ParseError> {
        let prev = std::mem::replace(&mut self.current, TokenKind::Eof);
        let tok = self.lexer.next_token()?;
        self.line = tok.span.line;
        self.column = tok.span.column;
        self.current = tok.kind;
        Ok(prev)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current) == std::mem::discriminant(kind)
    }

    fn eat(&mut self, kind: &TokenKind) -> Result<bool, ParseError> {
        if self.check(kind) {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<TokenKind, ParseError> {
        if self.check(kind) {
            self.advance()
        } else {
            Err(self.err(format!("expected {:?}, got {:?}", kind, self.current)))
        }
    }

    fn expect_name(&mut self) -> Result<String, ParseError> {
        match self.advance()? {
            TokenKind::Name(n) => Ok(n),
            other => Err(self.err(format!("expected name, got {other:?}"))),
        }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError {
            message: msg.into(),
            line: self.line,
            column: self.column,
        }
    }

    fn is_block_end(&self) -> bool {
        matches!(
            self.current,
            TokenKind::Else
                | TokenKind::Elseif
                | TokenKind::End
                | TokenKind::Until
                | TokenKind::Eof
        )
    }

    // --- Block & Statement parsing ---

    pub fn parse_block(&mut self) -> Result<Block, ParseError> {
        let mut stmts = Vec::new();
        let mut ret = None;

        loop {
            // Skip semicolons
            while self.eat(&TokenKind::Semicolon)? {}

            if self.is_block_end() {
                break;
            }

            if self.check(&TokenKind::Return) {
                self.advance()?;
                let values = if self.is_block_end() || self.check(&TokenKind::Semicolon) {
                    Vec::new()
                } else {
                    self.parse_expr_list()?
                };
                self.eat(&TokenKind::Semicolon)?;
                ret = Some(values);
                break;
            }

            stmts.push(self.parse_statement()?);
        }

        Ok(Block { stmts, ret })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        match &self.current {
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::Do => self.parse_do_block(),
            TokenKind::For => self.parse_for(),
            TokenKind::Repeat => self.parse_repeat(),
            TokenKind::Function => self.parse_function_stat(),
            TokenKind::Local => self.parse_local(),
            TokenKind::Break => {
                self.advance()?;
                Ok(Stmt::Break)
            }
            _ => self.parse_expr_stat(),
        }
    }

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&TokenKind::If)?;
        let condition = self.parse_expr()?;
        self.expect(&TokenKind::Then)?;
        let then_body = self.parse_block()?;

        let mut elseif_clauses = Vec::new();
        while self.eat(&TokenKind::Elseif)? {
            let cond = self.parse_expr()?;
            self.expect(&TokenKind::Then)?;
            let body = self.parse_block()?;
            elseif_clauses.push((cond, body));
        }

        let else_body = if self.eat(&TokenKind::Else)? {
            Some(self.parse_block()?)
        } else {
            None
        };

        self.expect(&TokenKind::End)?;
        Ok(Stmt::If {
            condition,
            then_body,
            elseif_clauses,
            else_body,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&TokenKind::While)?;
        let condition = self.parse_expr()?;
        self.expect(&TokenKind::Do)?;
        let body = self.parse_block()?;
        self.expect(&TokenKind::End)?;
        Ok(Stmt::While { condition, body })
    }

    fn parse_do_block(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&TokenKind::Do)?;
        let body = self.parse_block()?;
        self.expect(&TokenKind::End)?;
        Ok(Stmt::DoBlock { body })
    }

    fn parse_repeat(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&TokenKind::Repeat)?;
        let body = self.parse_block()?;
        self.expect(&TokenKind::Until)?;
        let condition = self.parse_expr()?;
        Ok(Stmt::Repeat { body, condition })
    }

    fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&TokenKind::For)?;
        let name = self.expect_name()?;

        if self.eat(&TokenKind::Assign)? {
            // Numeric for
            let start = self.parse_expr()?;
            self.expect(&TokenKind::Comma)?;
            let limit = self.parse_expr()?;
            let step = if self.eat(&TokenKind::Comma)? {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(&TokenKind::Do)?;
            let body = self.parse_block()?;
            self.expect(&TokenKind::End)?;
            Ok(Stmt::NumericFor {
                name,
                start,
                limit,
                step,
                body,
            })
        } else {
            // Generic for
            let mut names = vec![name];
            while self.eat(&TokenKind::Comma)? {
                names.push(self.expect_name()?);
            }
            self.expect(&TokenKind::In)?;
            let iterators = self.parse_expr_list()?;
            self.expect(&TokenKind::Do)?;
            let body = self.parse_block()?;
            self.expect(&TokenKind::End)?;
            Ok(Stmt::GenericFor {
                names,
                iterators,
                body,
            })
        }
    }

    fn parse_function_stat(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&TokenKind::Function)?;
        let name = self.parse_func_name()?;
        let (params, is_vararg, body) = self.parse_func_body()?;
        Ok(Stmt::FunctionDef {
            name,
            params,
            is_vararg,
            body,
        })
    }

    fn parse_func_name(&mut self) -> Result<FuncName, ParseError> {
        let mut parts = vec![self.expect_name()?];
        while self.eat(&TokenKind::Dot)? {
            parts.push(self.expect_name()?);
        }
        let method = if self.eat(&TokenKind::Colon)? {
            Some(self.expect_name()?)
        } else {
            None
        };
        Ok(FuncName { parts, method })
    }

    fn parse_func_body(&mut self) -> Result<(Vec<String>, bool, Block), ParseError> {
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        let mut is_vararg = false;

        if !self.check(&TokenKind::RParen) {
            if self.check(&TokenKind::DotDotDot) {
                self.advance()?;
                is_vararg = true;
            } else {
                params.push(self.expect_name()?);
                while self.eat(&TokenKind::Comma)? {
                    if self.check(&TokenKind::DotDotDot) {
                        self.advance()?;
                        is_vararg = true;
                        break;
                    }
                    params.push(self.expect_name()?);
                }
            }
        }

        self.expect(&TokenKind::RParen)?;
        let body = self.parse_block()?;
        self.expect(&TokenKind::End)?;
        Ok((params, is_vararg, body))
    }

    fn parse_local(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&TokenKind::Local)?;

        if self.check(&TokenKind::Function) {
            self.advance()?;
            let name = self.expect_name()?;
            let (params, is_vararg, body) = self.parse_func_body()?;
            return Ok(Stmt::LocalFunction {
                name,
                params,
                is_vararg,
                body,
            });
        }

        let mut names = vec![self.expect_name()?];
        while self.eat(&TokenKind::Comma)? {
            names.push(self.expect_name()?);
        }

        let values = if self.eat(&TokenKind::Assign)? {
            self.parse_expr_list()?
        } else {
            Vec::new()
        };

        Ok(Stmt::LocalAssign { names, values })
    }

    fn parse_expr_stat(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_suffixed_expr()?;

        // Check for assignment
        if self.check(&TokenKind::Assign) || self.check(&TokenKind::Comma) {
            let mut targets = vec![expr];
            while self.eat(&TokenKind::Comma)? {
                targets.push(self.parse_suffixed_expr()?);
            }
            self.expect(&TokenKind::Assign)?;
            let values = self.parse_expr_list()?;
            return Ok(Stmt::Assign { targets, values });
        }

        // Must be a function call
        match expr {
            Expr::FunctionCall(call) => Ok(Stmt::FunctionCall(call)),
            Expr::MethodCall {
                object,
                method,
                args,
            } => Ok(Stmt::MethodCall {
                object,
                method,
                args,
            }),
            _ => Err(self.err("expected assignment or function call")),
        }
    }

    // --- Expression parsing with Pratt precedence ---

    fn parse_expr_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut exprs = vec![self.parse_expr()?];
        while self.eat(&TokenKind::Comma)? {
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }

    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_sub_expr(0)
    }

    fn parse_sub_expr(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary_expr()?;

        while let Some((op, prec, right_assoc)) = self.current_binop() {
            if prec < min_prec {
                break;
            }
            self.advance()?;
            let next_min = if right_assoc { prec } else { prec + 1 };
            let rhs = self.parse_sub_expr(next_min)?;
            lhs = Expr::BinOp {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn current_binop(&self) -> Option<(BinOp, u8, bool)> {
        // Returns (op, precedence, right_associative)
        // Lua precedence (low to high): or=1 and=2 compare=3 concat=4 add=5 mul=6 pow=8
        // Unary is 7 (handled separately)
        match &self.current {
            TokenKind::Or => Some((BinOp::Or, 1, false)),
            TokenKind::And => Some((BinOp::And, 2, false)),
            TokenKind::Lt => Some((BinOp::Lt, 3, false)),
            TokenKind::Gt => Some((BinOp::Gt, 3, false)),
            TokenKind::Le => Some((BinOp::Le, 3, false)),
            TokenKind::Ge => Some((BinOp::Ge, 3, false)),
            TokenKind::Eq => Some((BinOp::Eq, 3, false)),
            TokenKind::Neq => Some((BinOp::Neq, 3, false)),
            TokenKind::DotDot => Some((BinOp::Concat, 4, true)),
            TokenKind::Plus => Some((BinOp::Add, 5, false)),
            TokenKind::Minus => Some((BinOp::Sub, 5, false)),
            TokenKind::Star => Some((BinOp::Mul, 6, false)),
            TokenKind::Slash => Some((BinOp::Div, 6, false)),
            TokenKind::Percent => Some((BinOp::Mod, 6, false)),
            TokenKind::Caret => Some((BinOp::Pow, 8, true)),
            _ => None,
        }
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, ParseError> {
        let op = match &self.current {
            TokenKind::Not => Some(UnOp::Not),
            TokenKind::Hash => Some(UnOp::Len),
            TokenKind::Minus => Some(UnOp::Neg),
            _ => None,
        };

        if let Some(op) = op {
            self.advance()?;
            let operand = self.parse_sub_expr(7)?; // unary precedence = 7
            return Ok(Expr::UnOp {
                op,
                operand: Box::new(operand),
            });
        }

        self.parse_suffixed_expr()
    }

    fn parse_suffixed_expr(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary_expr()?;

        loop {
            match &self.current {
                TokenKind::Dot => {
                    self.advance()?;
                    let name = self.expect_name()?;
                    expr = Expr::Field {
                        table: Box::new(expr),
                        name,
                    };
                }
                TokenKind::LBracket => {
                    self.advance()?;
                    let key = self.parse_expr()?;
                    self.expect(&TokenKind::RBracket)?;
                    expr = Expr::Index {
                        table: Box::new(expr),
                        key: Box::new(key),
                    };
                }
                TokenKind::Colon => {
                    self.advance()?;
                    let method = self.expect_name()?;
                    let args = self.parse_call_args()?;
                    expr = Expr::MethodCall {
                        object: Box::new(expr),
                        method,
                        args,
                    };
                }
                TokenKind::LParen | TokenKind::LBrace | TokenKind::StringLit(_) => {
                    let args = self.parse_call_args()?;
                    expr = Expr::FunctionCall(FunctionCall {
                        callee: Box::new(expr),
                        args,
                    });
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, ParseError> {
        match &self.current {
            TokenKind::Name(_) => {
                if let TokenKind::Name(n) = self.advance()? {
                    Ok(Expr::Name(n))
                } else {
                    unreachable!()
                }
            }
            TokenKind::LParen => {
                self.advance()?;
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            // Atoms that are not primary expressions but are simple expressions
            TokenKind::Nil => {
                self.advance()?;
                Ok(Expr::Nil)
            }
            TokenKind::True => {
                self.advance()?;
                Ok(Expr::True)
            }
            TokenKind::False => {
                self.advance()?;
                Ok(Expr::False)
            }
            TokenKind::Number(_) => {
                if let TokenKind::Number(n) = self.advance()? {
                    Ok(Expr::Number(n))
                } else {
                    unreachable!()
                }
            }
            TokenKind::StringLit(_) => {
                if let TokenKind::StringLit(s) = self.advance()? {
                    Ok(Expr::StringLit(s))
                } else {
                    unreachable!()
                }
            }
            TokenKind::DotDotDot => {
                self.advance()?;
                Ok(Expr::Vararg)
            }
            TokenKind::Function => {
                self.advance()?;
                let (params, is_vararg, body) = self.parse_func_body()?;
                Ok(Expr::FunctionDef {
                    params,
                    is_vararg,
                    body,
                })
            }
            TokenKind::LBrace => self.parse_table_constructor(),
            _ => Err(self.err(format!("unexpected token {:?}", self.current))),
        }
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        match &self.current {
            TokenKind::LParen => {
                self.advance()?;
                let args = if self.check(&TokenKind::RParen) {
                    Vec::new()
                } else {
                    self.parse_expr_list()?
                };
                self.expect(&TokenKind::RParen)?;
                Ok(args)
            }
            TokenKind::LBrace => {
                let table = self.parse_table_constructor()?;
                Ok(vec![table])
            }
            TokenKind::StringLit(_) => {
                if let TokenKind::StringLit(s) = self.advance()? {
                    Ok(vec![Expr::StringLit(s)])
                } else {
                    unreachable!()
                }
            }
            _ => Err(self.err("expected function arguments")),
        }
    }

    fn parse_table_constructor(&mut self) -> Result<Expr, ParseError> {
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.check(&TokenKind::LBracket) {
                // [expr] = expr
                self.advance()?;
                let key = self.parse_expr()?;
                self.expect(&TokenKind::RBracket)?;
                self.expect(&TokenKind::Assign)?;
                let value = self.parse_expr()?;
                fields.push(TableField::IndexedField { key, value });
            } else if matches!(&self.current, TokenKind::Name(_)) {
                // Could be `name = expr` or just `expr`
                // Need lookahead to distinguish
                let name = if let TokenKind::Name(n) = &self.current {
                    n.clone()
                } else {
                    unreachable!()
                };

                // Peek: if next is '=', it's a named field
                // We need to save state and try
                if self.peek_is_assign() {
                    self.advance()?; // consume name
                    self.expect(&TokenKind::Assign)?;
                    let value = self.parse_expr()?;
                    fields.push(TableField::NamedField { name, value });
                } else {
                    let value = self.parse_expr()?;
                    fields.push(TableField::PositionalField { value });
                }
            } else {
                let value = self.parse_expr()?;
                fields.push(TableField::PositionalField { value });
            }

            // Field separator: comma or semicolon (optional before closing brace)
            if !self.eat(&TokenKind::Comma)? && !self.eat(&TokenKind::Semicolon)? {
                break;
            }
        }

        self.expect(&TokenKind::RBrace)?;
        Ok(Expr::TableConstructor { fields })
    }

    /// Check if the token after the current Name is '='.
    /// This is a simple lookahead check for table constructors.
    fn peek_is_assign(&self) -> bool {
        // We need to peek at the next token. Since our lexer is streaming,
        // we check if current is Name and simulate by looking at the token after it.
        // Actually we need to check what comes after current token.
        // Since we haven't consumed current yet, we need a different approach.
        // Let's just check: we know current is Name(...). We need to see if the
        // next token from lexer would be Assign.
        // We'll use a simpler approach: create a temporary lexer copy.
        // But our lexer doesn't implement Clone easily.
        // Alternative: save lexer position and try.
        // For now, we'll do a simple check: peek into the source.

        // Actually, the simplest approach is just to try parsing as expr.
        // But we need lookahead. Let's use a manual check on raw source bytes.

        // Save lexer state
        let saved_pos = self.lexer_pos();

        // We can't easily peek without mutating. Let's use a different approach:
        // Since current is Name, check if there's an '=' (not '==') next.
        // This requires peeking one token ahead.
        // Let's just accept the limitation and restructure:
        // We'll use a two-token check approach.

        // For simplicity, we check raw bytes after skipping whitespace
        let mut pos = saved_pos;
        let source = self.lexer_source();
        // Skip whitespace
        while pos < source.len()
            && (source[pos] == b' '
                || source[pos] == b'\t'
                || source[pos] == b'\r'
                || source[pos] == b'\n')
        {
            pos += 1;
        }
        // Check for '=' but not '=='
        pos < source.len() && source[pos] == b'=' && source.get(pos + 1) != Some(&b'=')
    }

    fn lexer_pos(&self) -> usize {
        self.lexer.pos()
    }

    fn lexer_source(&self) -> &[u8] {
        self.lexer.source()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_block(source: &str) -> Block {
        let mut parser = Parser::new(source).expect("parser init");
        parser.parse_block().expect("parse failed")
    }

    fn parse_expr_str(source: &str) -> Expr {
        let wrapped = format!("return {source}");
        let block = parse_block(&wrapped);
        let ret = block.ret.expect("expected return");
        assert_eq!(ret.len(), 1);
        ret.into_iter().next().unwrap()
    }

    #[test]
    fn parse_local_assign() {
        let block = parse_block("local x = 1");
        assert_eq!(block.stmts.len(), 1);
        match &block.stmts[0] {
            Stmt::LocalAssign { names, values } => {
                assert_eq!(names, &["x"]);
                assert_eq!(values.len(), 1);
                assert_eq!(values[0], Expr::Number(1.0));
            }
            _ => panic!("expected LocalAssign"),
        }
    }

    #[test]
    fn parse_assignment() {
        let block = parse_block("x = x + 1");
        match &block.stmts[0] {
            Stmt::Assign { targets, values } => {
                assert_eq!(targets.len(), 1);
                assert_eq!(values.len(), 1);
            }
            _ => panic!("expected Assign"),
        }
    }

    #[test]
    fn parse_if_statement() {
        let block = parse_block("if x then y() end");
        match &block.stmts[0] {
            Stmt::If {
                condition,
                then_body,
                elseif_clauses,
                else_body,
            } => {
                assert_eq!(*condition, Expr::Name("x".into()));
                assert!(!then_body.stmts.is_empty());
                assert!(elseif_clauses.is_empty());
                assert!(else_body.is_none());
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_while_loop() {
        let block = parse_block("while true do break end");
        match &block.stmts[0] {
            Stmt::While { condition, body } => {
                assert_eq!(*condition, Expr::True);
                assert_eq!(body.stmts.len(), 1);
                assert!(matches!(body.stmts[0], Stmt::Break));
            }
            _ => panic!("expected While"),
        }
    }

    #[test]
    fn parse_numeric_for() {
        let block = parse_block("for i = 1, 10 do end");
        match &block.stmts[0] {
            Stmt::NumericFor {
                name,
                start,
                limit,
                step,
                ..
            } => {
                assert_eq!(name, "i");
                assert_eq!(*start, Expr::Number(1.0));
                assert_eq!(*limit, Expr::Number(10.0));
                assert!(step.is_none());
            }
            _ => panic!("expected NumericFor"),
        }
    }

    #[test]
    fn parse_generic_for() {
        let block = parse_block("for k, v in pairs(t) do end");
        match &block.stmts[0] {
            Stmt::GenericFor { names, .. } => {
                assert_eq!(names, &["k", "v"]);
            }
            _ => panic!("expected GenericFor"),
        }
    }

    #[test]
    fn parse_function_def() {
        let block = parse_block("function f(a, b) return a + b end");
        match &block.stmts[0] {
            Stmt::FunctionDef { name, params, .. } => {
                assert_eq!(name.parts, vec!["f"]);
                assert_eq!(params, &["a", "b"]);
            }
            _ => panic!("expected FunctionDef"),
        }
    }

    #[test]
    fn parse_local_function() {
        let block = parse_block("local function f() end");
        match &block.stmts[0] {
            Stmt::LocalFunction { name, .. } => {
                assert_eq!(name, "f");
            }
            _ => panic!("expected LocalFunction"),
        }
    }

    #[test]
    fn parse_return() {
        let block = parse_block("return 1, 2, 3");
        assert_eq!(
            block.ret,
            Some(vec![
                Expr::Number(1.0),
                Expr::Number(2.0),
                Expr::Number(3.0),
            ])
        );
    }

    #[test]
    fn parse_table_constructor() {
        let expr = parse_expr_str("{ 1, 2, x = 3, [4] = 5 }");
        match expr {
            Expr::TableConstructor { fields } => {
                assert_eq!(fields.len(), 4);
            }
            _ => panic!("expected TableConstructor"),
        }
    }

    #[test]
    fn precedence_mul_over_add() {
        let expr = parse_expr_str("1 + 2 * 3");
        match expr {
            Expr::BinOp {
                op: BinOp::Add,
                right,
                ..
            } => match *right {
                Expr::BinOp { op: BinOp::Mul, .. } => {}
                _ => panic!("expected Mul on right"),
            },
            _ => panic!("expected Add at top"),
        }
    }

    #[test]
    fn precedence_pow_right_assoc() {
        let expr = parse_expr_str("2 ^ 3 ^ 4");
        match expr {
            Expr::BinOp {
                op: BinOp::Pow,
                right,
                ..
            } => match *right {
                Expr::BinOp { op: BinOp::Pow, .. } => {}
                _ => panic!("expected Pow on right"),
            },
            _ => panic!("expected Pow at top"),
        }
    }

    #[test]
    fn parse_method_call() {
        let block = parse_block("a:b(c)");
        match &block.stmts[0] {
            Stmt::MethodCall { method, args, .. } => {
                assert_eq!(method, "b");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("expected MethodCall"),
        }
    }

    #[test]
    fn parse_repeat_until() {
        let block = parse_block("repeat x = 1 until true");
        match &block.stmts[0] {
            Stmt::Repeat { condition, .. } => {
                assert_eq!(*condition, Expr::True);
            }
            _ => panic!("expected Repeat"),
        }
    }
}
