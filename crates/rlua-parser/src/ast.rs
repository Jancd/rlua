/// Lua 5.1 AST node types.

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub ret: Option<Vec<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Assign {
        targets: Vec<Expr>,
        values: Vec<Expr>,
    },
    LocalAssign {
        names: Vec<String>,
        values: Vec<Expr>,
    },
    DoBlock {
        body: Block,
    },
    While {
        condition: Expr,
        body: Block,
    },
    Repeat {
        body: Block,
        condition: Expr,
    },
    If {
        condition: Expr,
        then_body: Block,
        elseif_clauses: Vec<(Expr, Block)>,
        else_body: Option<Block>,
    },
    NumericFor {
        name: String,
        start: Expr,
        limit: Expr,
        step: Option<Expr>,
        body: Block,
    },
    GenericFor {
        names: Vec<String>,
        iterators: Vec<Expr>,
        body: Block,
    },
    FunctionCall(FunctionCall),
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    LocalFunction {
        name: String,
        params: Vec<String>,
        is_vararg: bool,
        body: Block,
    },
    FunctionDef {
        name: FuncName,
        params: Vec<String>,
        is_vararg: bool,
        body: Block,
    },
    Break,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncName {
    pub parts: Vec<String>,
    pub method: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Nil,
    True,
    False,
    Number(f64),
    StringLit(String),
    Vararg,
    Name(String),
    Index {
        table: Box<Expr>,
        key: Box<Expr>,
    },
    Field {
        table: Box<Expr>,
        name: String,
    },
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnOp {
        op: UnOp,
        operand: Box<Expr>,
    },
    FunctionCall(FunctionCall),
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    FunctionDef {
        params: Vec<String>,
        is_vararg: bool,
        body: Block,
    },
    TableConstructor {
        fields: Vec<TableField>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TableField {
    IndexedField { key: Expr, value: Expr },
    NamedField { name: String, value: Expr },
    PositionalField { value: Expr },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Concat,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    Len,
}
