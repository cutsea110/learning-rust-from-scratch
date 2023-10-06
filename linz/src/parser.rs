//! BNF
//!
//! <Q> := lin | un
//! <B> := true | false
//! <V> := [a-zA-Z_][a-zA-Z0-9_]*
//! <E> := <V>
//!      | <Q> <B>
//!      | if <E> { <E> } else { <E> }
//!      | <Q> fn <V> : <T> { <E> }
//!      | (<E> <E>)
//!      | <Q> <<E>, <E>>
//!      | split <E> as <V>,<V> { <E> }
//!      | free <V> ; <E>
//! <P> := bool
//!      | (<T> * <T>)
//!      | (<T> -> <T>)
//! <T> := <Q> <P>
//! <Y> := epmty
//!      | <Y> , <V> : <T>
use crate::lang::*;
use crate::parser_combinator::*;

fn qual<'a>() -> impl Parser<'a, Qual> {
    let lin = literal("lin").map(|_| Qual::Lin);
    let un = literal("un").map(|_| Qual::Un);
    lexeme(lin.or_else(un))
}

fn bool<'a>() -> impl Parser<'a, ValExpr> {
    let t = literal("true").map(|_| ValExpr::Bool(true));
    let f = literal("false").map(|_| ValExpr::Bool(false));
    lexeme(t.or_else(f))
}

fn variable(input: &str) -> ParseResult<String> {
    let mut matched = String::new();
    let mut chars = input.chars();

    match chars.next() {
        Some(next) if next.is_alphabetic() || next == '_' => matched.push(next),
        _ => return Err(input),
    }

    while let Some(next) = chars.next() {
        if next.is_alphabetic() || next == '_' {
            matched.push(next);
        } else {
            break;
        }
    }

    let next_index = matched.len();
    Ok((&input[next_index..], matched))
}

fn expr<'a>() -> impl Parser<'a, Expr> {
    let var = variable.map(|s| Expr::Var(s));
    let qbool = qual()
        .join(bool())
        .map(|(q, b)| Expr::QVal(QValExpr { qual: q, val: b }));
    let if_expr = literal("if")
        .skip(expr())
        .join(bracket(lexeme(char('{')), expr(), lexeme(char('}'))))
        .with(literal("else"))
        .join(bracket(lexeme(char('{')), expr(), lexeme(char('}'))))
        .map(|((c, t), e)| {
            Expr::If(IfExpr {
                cond_expr: Box::new(c),
                then_expr: Box::new(t),
                else_expr: Box::new(e),
            })
        });
    let fn_expr = qual()
        .with(lexeme(literal("fn")))
        .join(variable)
        .with(lexeme(char(':')))
        .join(type_expr())
        .join(bracket(lexeme(char('{')), expr(), lexeme(char('}'))))
        .map(|(((q, v), t), e)| {
            Expr::QVal(QValExpr {
                qual: q,
                val: ValExpr::Fun(FnExpr {
                    var: v,
                    ty: t,
                    expr: Box::new(e),
                }),
            })
        });
    let app = parens(expr().join(expr())).map(|(e1, e2)| {
        Expr::App(AppExpr {
            expr1: Box::new(e1),
            expr2: Box::new(e2),
        })
    });
    let tuple = qual()
        .join(bracket(
            lexeme(char('<')),
            expr().with(lexeme(char(','))).join(expr()),
            lexeme(char('>')),
        ))
        .map(|(q, (e1, e2))| {
            Expr::QVal(QValExpr {
                qual: q,
                val: ValExpr::Pair(Box::new(e1), Box::new(e2)),
            })
        });
    let split_expr = literal("split")
        .skip(expr())
        .with(literal("as"))
        .join(variable.with(lexeme(char(','))).join(variable))
        .join(bracket(lexeme(char('{')), expr(), lexeme(char('}'))))
        .map(|((e, (v1, v2)), e1)| {
            Expr::Split(SplitExpr {
                expr: Box::new(e),
                left: v1,
                right: v2,
                body: Box::new(e1),
            })
        });
    let free_stmt = literal("free")
        .skip(variable)
        .with(lexeme(char(';')))
        .join(expr())
        .map(|(v, e)| {
            Expr::Free(FreeExpr {
                var: v,
                expr: Box::new(e),
            })
        });

    var.or_else(qbool)
        .or_else(if_expr)
        .or_else(fn_expr)
        .or_else(app)
        .or_else(tuple)
        .or_else(split_expr)
        .or_else(free_stmt)
}

fn prim_type<'a>() -> impl Parser<'a, PrimType> {
    let bool = lexeme(literal("bool")).map(|_| PrimType::Bool);
    let tuple = parens(type_expr().with(lexeme(char('*'))).join(type_expr()))
        .map(|(t1, t2)| PrimType::Pair(Box::new(t1), Box::new(t2)));
    let arrow = parens(type_expr().with(lexeme(literal("->"))).join(type_expr()))
        .map(|(t1, t2)| PrimType::Arrow(Box::new(t1), Box::new(t2)));

    bool.or_else(tuple).or_else(arrow)
}

fn type_expr<'a>() -> impl Parser<'a, TypeExpr> {
    qual()
        .join(prim_type())
        .map(|(q, p)| TypeExpr { qual: q, prim: p })
}
