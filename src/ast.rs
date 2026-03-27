/// AST types for the Clojure-to-Rust transpiler

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<TopLevel>,
}

#[derive(Debug, Clone)]
pub enum TopLevel {
    Use(UsePath),
    ExternCrate(String),
    Def(Def),
    Defn(Defn),
    DefStruct(DefStruct),
    DefEnum(DefEnum),
    DefTrait(DefTrait),
    Impl(Impl),
    Mod(Mod),
    Attr(String, Box<TopLevel>),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct UsePath {
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct Def {
    pub is_pub: bool,
    pub mutable: bool,
    pub name: String,
    pub type_ann: Option<String>,
    pub value: Expr,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct Defn {
    pub is_pub: bool,
    pub name: String,
    pub generics: Vec<String>,
    pub params: Vec<Param>,
    pub ret_type: Option<String>,
    pub body: Vec<Expr>,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<String>,
    pub is_ref: bool,
    pub is_mut_ref: bool,
    pub is_mut: bool,
}

#[derive(Debug, Clone)]
pub struct DefStruct {
    pub is_pub: bool,
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<StructField>,
    pub derives: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub is_pub: bool,
    pub name: String,
    pub type_ann: String,
}

#[derive(Debug, Clone)]
pub struct DefEnum {
    pub is_pub: bool,
    pub name: String,
    pub generics: Vec<String>,
    pub variants: Vec<EnumVariant>,
    pub derives: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: VariantFields,
}

#[derive(Debug, Clone)]
pub enum VariantFields {
    Unit,
    Tuple(Vec<String>),
    Struct(Vec<StructField>),
}

#[derive(Debug, Clone)]
pub struct DefTrait {
    pub is_pub: bool,
    pub name: String,
    pub generics: Vec<String>,
    pub methods: Vec<Defn>,
}

#[derive(Debug, Clone)]
pub struct Impl {
    pub type_name: String,
    pub generics: Vec<String>,
    pub trait_name: Option<String>,
    pub methods: Vec<Defn>,
}

#[derive(Debug, Clone)]
pub struct Mod {
    pub is_pub: bool,
    pub name: String,
    pub items: Vec<TopLevel>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Integer(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Char(char),
    Nil,
    Symbol(String),
    Keyword(String),

    // Collections
    Vec(Vec<Expr>),
    HashMap(Vec<(Expr, Expr)>),
    Tuple(Vec<Expr>),

    // Bindings
    Let {
        bindings: Vec<LetBinding>,
        body: Vec<Expr>,
    },

    // Control flow
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Option<Box<Expr>>,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Do(Vec<Expr>),
    Block(Vec<Expr>),
    Loop {
        bindings: Vec<LetBinding>,
        body: Vec<Expr>,
    },
    Recur(Vec<Expr>),
    For {
        binding: String,
        iter: Box<Expr>,
        body: Vec<Expr>,
    },
    While {
        cond: Box<Expr>,
        body: Vec<Expr>,
    },

    // Functions
    Fn {
        params: Vec<Param>,
        ret_type: Option<String>,
        body: Vec<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    MacroCall {
        name: String,
        args: Vec<Expr>,
    },

    // Rust interop
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
    },
    StaticCall {
        type_name: String,
        method: String,
        args: Vec<Expr>,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    },

    // Operators
    BinOp {
        op: BinOpKind,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOpKind,
        operand: Box<Expr>,
    },

    // Assignment
    Set {
        target: Box<Expr>,
        value: Box<Expr>,
    },

    // References
    Ref(Box<Expr>),
    RefMut(Box<Expr>),
    Deref(Box<Expr>),

    // Type operations
    As {
        expr: Box<Expr>,
        type_name: String,
    },

    // Range
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
    },

    // Return / break
    Return(Option<Box<Expr>>),
    Break(Option<Box<Expr>>),
    Continue,

    // Await
    Await(Box<Expr>),

    // Try (?)
    Try(Box<Expr>),

    // Raw Rust escape hatch
    RawRust(String),

    // Index access
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub mutable: bool,
    pub type_ann: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Literal(Expr),
    Binding(String),
    Tuple(Vec<Pattern>),
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    TupleStruct {
        name: String,
        fields: Vec<Pattern>,
    },
    Ref(Box<Pattern>),
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub enum UnaryOpKind {
    Not,
    Neg,
}

impl BinOpKind {
    pub fn to_rust(&self) -> &'static str {
        match self {
            BinOpKind::Add => "+",
            BinOpKind::Sub => "-",
            BinOpKind::Mul => "*",
            BinOpKind::Div => "/",
            BinOpKind::Rem => "%",
            BinOpKind::Eq => "==",
            BinOpKind::NotEq => "!=",
            BinOpKind::Lt => "<",
            BinOpKind::Gt => ">",
            BinOpKind::LtEq => "<=",
            BinOpKind::GtEq => ">=",
            BinOpKind::And => "&&",
            BinOpKind::Or => "||",
            BinOpKind::BitAnd => "&",
            BinOpKind::BitOr => "|",
            BinOpKind::BitXor => "^",
            BinOpKind::Shl => "<<",
            BinOpKind::Shr => ">>",
        }
    }
}

impl UnaryOpKind {
    pub fn to_rust(&self) -> &'static str {
        match self {
            UnaryOpKind::Not => "!",
            UnaryOpKind::Neg => "-",
        }
    }
}
