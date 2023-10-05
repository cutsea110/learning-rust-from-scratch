use std::fmt;

/// 抽象構文木
#[derive(Debug)]
pub enum Expr {
    Let(LetExpr),
    If(IfExpr),
    Split(SplitExpr),
    Free(FreeExpr),
    App(AppExpr),
    Var(String),
    QVal(QValExpr),
}

/// let 式
#[derive(Debug)]
pub struct LetExpr {
    pub var: String,
    pub ty: TypeExpr,
    pub expr1: Box<Expr>,
    pub expr2: Box<Expr>,
}

/// if 式
#[derive(Debug)]
pub struct IfExpr {
    pub cond_expr: Box<Expr>,
    pub then_expr: Box<Expr>,
    pub else_expr: Box<Expr>,
}

/// split 式
#[derive(Debug)]
pub struct SplitExpr {
    pub expr: Box<Expr>,
    pub left: String,
    pub right: String,
    pub body: Box<Expr>,
}

/// free 文
#[derive(Debug)]
pub struct FreeExpr {
    pub var: String,
    pub expr: Box<Expr>,
}

/// 関数適用
#[derive(Debug)]
pub struct AppExpr {
    pub expr1: Box<Expr>,
    pub expr2: Box<Expr>,
}

/// 修飾子付き値
#[derive(Debug)]
pub struct QValExpr {
    pub qual: Qual,
    pub val: ValExpr,
}

/// 値, 真偽値, 対, 関数(λ抽象)
#[derive(Debug)]
pub enum ValExpr {
    Bool(bool),
    Pair(Box<Expr>, Box<Expr>),
    Fun(FnExpr),
}

/// 修飾子
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Qual {
    Lin, // 線形型
    Un,  // 制約のない一般的な型
}

/// 関数
#[derive(Debug)]
pub struct FnExpr {
    pub var: String,
    pub ty: TypeExpr,
    pub expr: Box<Expr>,
}

/// 修飾子付き型
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TypeExpr {
    pub qual: Qual,
    pub prim: PrimType,
}
impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.qual {
            Qual::Lin => write!(f, "lin {}", self.prim),
            Qual::Un => write!(f, "un {}", self.prim),
        }
    }
}

/// プリミティブ型
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum PrimType {
    Bool,
    Pair(Box<TypeExpr>, Box<TypeExpr>),
    Arrow(Box<TypeExpr>, Box<TypeExpr>),
}
impl fmt::Display for PrimType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimType::Bool => write!(f, "bool"),
            PrimType::Pair(t1, t2) => write!(f, "{t1} * {t2}"),
            PrimType::Arrow(t1, t2) => write!(f, "{t1} -> {t2}"),
        }
    }
}
