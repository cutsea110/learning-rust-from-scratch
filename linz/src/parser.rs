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

fn infix_pair<'a, P1, P2, Sep, R1, R2, O>(
    parser1: P1,
    sep: Sep,
    parser2: P2,
) -> impl Parser<'a, (R1, R2)>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
    Sep: Parser<'a, O>,
{
    move |input| match parser1.parse(input) {
        Ok((next_input, result1)) => match sep.parse(next_input) {
            Ok((next_input, _)) => match parser2.parse(next_input) {
                Ok((final_input, result2)) => Ok((final_input, (result1, result2))),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        },
        Err(e) => Err(e),
    }
}

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

fn if_expr<'a>() -> impl Parser<'a, IfExpr> {
    literal("if")
        .skip(expr())
        .join(braces(expr()))
        .with(lexeme(literal("else")))
        .join(braces(expr()))
        .map(|((c, t), e)| IfExpr {
            cond_expr: Box::new(c),
            then_expr: Box::new(t),
            else_expr: Box::new(e),
        })
}

fn fn_expr<'a>() -> impl Parser<'a, QValExpr> {
    qual()
        .with(lexeme(literal("fn")))
        .join(infix_pair(variable, lexeme(char(':')), type_expr()))
        .join(braces(expr()))
        .map(|((q, (v, t)), e)| QValExpr {
            qual: q,
            val: ValExpr::Fun(FnExpr {
                var: v,
                ty: t,
                expr: Box::new(e),
            }),
        })
}

fn app_expr<'a>() -> impl Parser<'a, AppExpr> {
    parens(expr().join(expr())).map(|(e1, e2)| AppExpr {
        expr1: Box::new(e1),
        expr2: Box::new(e2),
    })
}

fn tuple_expr<'a>() -> impl Parser<'a, QValExpr> {
    qual()
        .join(angles(infix_pair(expr(), lexeme(char(',')), expr())))
        .map(|(q, (e1, e2))| QValExpr {
            qual: q,
            val: ValExpr::Pair(Box::new(e1), Box::new(e2)),
        })
}

fn split_expr<'a>() -> impl Parser<'a, SplitExpr> {
    literal("split")
        .skip(expr())
        .with(literal("as"))
        .join(infix_pair(variable, lexeme(char(',')), variable))
        .join(braces(expr()))
        .map(|((e, (v1, v2)), e1)| SplitExpr {
            expr: Box::new(e),
            left: v1,
            right: v2,
            body: Box::new(e1),
        })
}

fn free_stmt<'a>() -> impl Parser<'a, FreeExpr> {
    literal("free")
        .skip(infix_pair(variable, lexeme(char(';')), expr()))
        .map(|(v, e)| FreeExpr {
            var: v,
            expr: Box::new(e),
        })
}

fn expr<'a>() -> impl Parser<'a, Expr> {
    let var = variable.map(|s| Expr::Var(s));
    let qbool = qual()
        .join(bool())
        .map(|(q, b)| Expr::QVal(QValExpr { qual: q, val: b }));
    let if_expr = if_expr().map(|e| Expr::If(e));
    let fn_expr = fn_expr().map(|e| Expr::QVal(e));
    let app = app_expr().map(|e| Expr::App(e));
    let tuple = tuple_expr().map(|e| Expr::QVal(e));
    let split_expr = split_expr().map(|e| Expr::Split(e));
    let free_stmt = free_stmt().map(|e| Expr::Free(e));

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
    let tuple = parens(infix_pair(type_expr(), lexeme(char('*')), type_expr()))
        .map(|(t1, t2)| PrimType::Pair(Box::new(t1), Box::new(t2)));
    let arrow = parens(infix_pair(type_expr(), lexeme(literal("->")), type_expr()))
        .map(|(t1, t2)| PrimType::Arrow(Box::new(t1), Box::new(t2)));

    bool.or_else(tuple).or_else(arrow)
}

fn type_expr<'a>() -> impl Parser<'a, TypeExpr> {
    qual()
        .join(prim_type())
        .map(|(q, p)| TypeExpr { qual: q, prim: p })
}
