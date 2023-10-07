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
#[cfg(test)]
mod qual_test {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(qual().parse("lin"), Ok(("", Qual::Lin)));
        assert_eq!(qual().parse(" lin"), Ok(("", Qual::Lin)));
        assert_eq!(qual().parse("un"), Ok(("", Qual::Un)));
        assert_eq!(qual().parse(" un"), Ok(("", Qual::Un)));
        assert_eq!(qual().parse("foo"), Err("foo"));
    }
}

fn bool<'a>() -> impl Parser<'a, ValExpr> {
    let t = literal("true").map(|_| ValExpr::Bool(true));
    let f = literal("false").map(|_| ValExpr::Bool(false));
    lexeme(t.or_else(f))
}
#[cfg(test)]
mod bool_test {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(bool().parse("true"), Ok(("", ValExpr::Bool(true))));
        assert_eq!(bool().parse(" true"), Ok(("", ValExpr::Bool(true))));
        assert_eq!(bool().parse("false"), Ok(("", ValExpr::Bool(false))));
        assert_eq!(bool().parse(" false"), Ok(("", ValExpr::Bool(false))));
        assert_eq!(bool().parse("foo"), Err("foo"));
    }
}

fn variable(input: &str) -> ParseResult<String> {
    let mut matched = String::new();
    let mut chars = input.chars();

    match chars.next() {
        Some(next) if next.is_alphabetic() || next == '_' => matched.push(next),
        _ => return Err(input),
    }

    while let Some(next) = chars.next() {
        if next.is_alphanumeric() || next == '_' {
            matched.push(next);
        } else {
            break;
        }
    }

    let next_index = matched.len();
    Ok((&input[next_index..], matched))
}
#[cfg(test)]
mod variable_test {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(variable("foo"), Ok(("", "foo".to_string())));
        assert_eq!(variable("foo_bar"), Ok(("", "foo_bar".to_string())));
        assert_eq!(variable("foo1"), Ok(("", "foo1".to_string())));
        assert_eq!(variable("1foo"), Err("1foo"));
    }
}

fn qual_bool<'a>() -> impl Parser<'a, QValExpr> {
    qual().and_then(|qual| bool().map(move |val| QValExpr { qual, val }))
}
#[cfg(test)]
mod qual_bool_test {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            qual_bool().parse("lin true"),
            Ok((
                "",
                QValExpr {
                    qual: Qual::Lin,
                    val: ValExpr::Bool(true)
                }
            ))
        );
        assert_eq!(
            qual_bool().parse(" lin  true"),
            Ok((
                "",
                QValExpr {
                    qual: Qual::Lin,
                    val: ValExpr::Bool(true)
                }
            ))
        );
        assert_eq!(
            qual_bool().parse("un false"),
            Ok((
                "",
                QValExpr {
                    qual: Qual::Un,
                    val: ValExpr::Bool(false)
                }
            ))
        );
        assert_eq!(
            qual_bool().parse(" un  false"),
            Ok((
                "",
                QValExpr {
                    qual: Qual::Un,
                    val: ValExpr::Bool(false)
                }
            ))
        );
        assert_eq!(qual_bool().parse("un"), Err(""));
        assert_eq!(qual_bool().parse("lin"), Err(""));
        assert_eq!(qual_bool().parse("true"), Err("true"));
        assert_eq!(qual_bool().parse("false"), Err("false"));
        assert_eq!(qual_bool().parse("foo"), Err("foo"));
    }
}

fn if_expr<'a>() -> impl Parser<'a, IfExpr> {
    let if_ = lexeme(literal("if")).skip(expr());
    let then_ = braces(expr());
    let else_ = lexeme(literal("else")).skip(braces(expr()));

    move |input| {
        if_.parse(input).and_then(|(next_input, c)| {
            then_.parse(next_input).and_then(|(next_input, t)| {
                else_.parse(next_input).and_then(|(next_input, e)| {
                    Ok((
                        next_input,
                        IfExpr {
                            cond_expr: Box::new(c),
                            then_expr: Box::new(t),
                            else_expr: Box::new(e),
                        },
                    ))
                })
            })
        })
    }
}
#[cfg(test)]
mod if_expr_test {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            if_expr().parse("if lin true { un false } else { lin true }"),
            Ok((
                "",
                IfExpr {
                    cond_expr: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Lin,
                        val: ValExpr::Bool(true)
                    })),
                    then_expr: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Un,
                        val: ValExpr::Bool(false)
                    })),
                    else_expr: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Lin,
                        val: ValExpr::Bool(true)
                    }))
                }
            ))
        );
    }
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
#[cfg(test)]
mod fn_expr_test {
    use super::*;

    // #[test]
    fn test() {
        assert_eq!(
            fn_expr().parse("lin fn foo: un bool { un false }"),
            Ok((
                "",
                QValExpr {
                    qual: Qual::Lin,
                    val: ValExpr::Fun(FnExpr {
                        var: "foo".to_string(),
                        ty: TypeExpr {
                            qual: Qual::Un,
                            prim: PrimType::Bool
                        },
                        expr: Box::new(Expr::QVal(QValExpr {
                            qual: Qual::Un,
                            val: ValExpr::Bool(false)
                        }))
                    })
                }
            ))
        );
    }
}

fn app_expr<'a>() -> impl Parser<'a, AppExpr> {
    parens(expr().join(expr())).map(|(e1, e2)| AppExpr {
        expr1: Box::new(e1),
        expr2: Box::new(e2),
    })
}
#[cfg(test)]
mod app_expr_test {
    use super::*;

    // #[test]
    fn test() {
        assert_eq!(
            app_expr().parse("(lin fn foo: un bool { un false }) (un true)"),
            Ok((
                "",
                AppExpr {
                    expr1: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Lin,
                        val: ValExpr::Fun(FnExpr {
                            var: "foo".to_string(),
                            ty: TypeExpr {
                                qual: Qual::Un,
                                prim: PrimType::Bool
                            },
                            expr: Box::new(Expr::QVal(QValExpr {
                                qual: Qual::Un,
                                val: ValExpr::Bool(false)
                            }))
                        })
                    })),
                    expr2: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Un,
                        val: ValExpr::Bool(true)
                    }))
                }
            ))
        );
    }
}

fn tuple_expr<'a>() -> impl Parser<'a, QValExpr> {
    qual()
        .join(angles(infix_pair(expr(), lexeme(char(',')), expr())))
        .map(|(q, (e1, e2))| QValExpr {
            qual: q,
            val: ValExpr::Pair(Box::new(e1), Box::new(e2)),
        })
}
#[cfg(test)]
mod tuple_expr_test {
    use super::*;

    // #[test]
    fn test() {
        assert_eq!(
            tuple_expr().parse("lin <un true, lin false>"),
            Ok((
                "",
                QValExpr {
                    qual: Qual::Lin,
                    val: ValExpr::Pair(
                        Box::new(Expr::QVal(QValExpr {
                            qual: Qual::Un,
                            val: ValExpr::Bool(true)
                        })),
                        Box::new(Expr::QVal(QValExpr {
                            qual: Qual::Lin,
                            val: ValExpr::Bool(false)
                        }))
                    )
                }
            ))
        );
    }
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
#[cfg(test)]
mod split_expr_test {
    use super::*;

    // #[test]
    fn test() {
        assert_eq!(
            split_expr().parse("split lin <un true, lin false> as foo, bar { un false }"),
            Ok((
                "",
                SplitExpr {
                    expr: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Lin,
                        val: ValExpr::Pair(
                            Box::new(Expr::QVal(QValExpr {
                                qual: Qual::Un,
                                val: ValExpr::Bool(true)
                            })),
                            Box::new(Expr::QVal(QValExpr {
                                qual: Qual::Lin,
                                val: ValExpr::Bool(false)
                            }))
                        )
                    })),
                    left: "foo".to_string(),
                    right: "bar".to_string(),
                    body: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Un,
                        val: ValExpr::Bool(false)
                    }))
                }
            ))
        );
    }
}

fn free_stmt<'a>() -> impl Parser<'a, FreeExpr> {
    literal("free")
        .skip(infix_pair(variable, lexeme(char(';')), expr()))
        .map(|(v, e)| FreeExpr {
            var: v,
            expr: Box::new(e),
        })
}
#[cfg(test)]
mod free_stmt_test {
    use super::*;

    // #[test]
    fn test() {
        assert_eq!(
            free_stmt().parse("free foo; un false"),
            Ok((
                "",
                FreeExpr {
                    var: "foo".to_string(),
                    expr: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Un,
                        val: ValExpr::Bool(false)
                    }))
                }
            ))
        );
    }
}

fn expr<'a>() -> impl Parser<'a, Expr> {
    let qbool = qual_bool().map(|e| Expr::QVal(e));
    // let if_expr = if_expr().map(|e| Expr::If(e));
    // let fn_expr = fn_expr().map(|e| Expr::QVal(e));
    // let app = app_expr().map(|e| Expr::App(e));
    // let tuple = tuple_expr().map(|e| Expr::QVal(e));
    // let split_expr = split_expr().map(|e| Expr::Split(e));
    // let free_stmt = free_stmt().map(|e| Expr::Free(e));
    let var = variable.map(|s| Expr::Var(s));

    qbool
        // .or_else(if_expr)
        // .or_else(fn_expr)
        // .or_else(app)
        // .or_else(tuple)
        // .or_else(split_expr)
        // .or_else(free_stmt)
        .or_else(var)
}
#[cfg(test)]
mod expr_test {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(expr().parse("x"), Ok(("", Expr::Var("x".to_string()))));
        assert_eq!(expr().parse("unx"), Ok(("", Expr::Var("unx".to_string()))));
        assert_eq!(
            expr().parse("linz"),
            Ok(("", Expr::Var("linz".to_string())))
        );
        assert_eq!(
            expr().parse("un true"),
            Ok((
                "",
                Expr::QVal(QValExpr {
                    qual: Qual::Un,
                    val: ValExpr::Bool(true)
                })
            ))
        );
    }
}

fn prim_type<'a>() -> impl Parser<'a, PrimType> {
    let bool = lexeme(literal("bool")).map(|_| PrimType::Bool);
    let tuple = parens(infix_pair(type_expr(), lexeme(char('*')), type_expr()))
        .map(|(t1, t2)| PrimType::Pair(Box::new(t1), Box::new(t2)));
    let arrow = parens(infix_pair(type_expr(), lexeme(literal("->")), type_expr()))
        .map(|(t1, t2)| PrimType::Arrow(Box::new(t1), Box::new(t2)));

    bool.or_else(tuple).or_else(arrow)
}
#[cfg(test)]
mod prim_type {
    use super::*;

    // #[test]
    fn test() {
        assert_eq!(prim_type().parse("bool"), Ok(("", PrimType::Bool)));
        assert_eq!(
            prim_type().parse("(un bool * lin bool)"),
            Ok((
                "",
                PrimType::Pair(
                    Box::new(TypeExpr {
                        qual: Qual::Un,
                        prim: PrimType::Bool
                    }),
                    Box::new(TypeExpr {
                        qual: Qual::Lin,
                        prim: PrimType::Bool
                    })
                )
            ))
        );
        assert_eq!(
            prim_type().parse("(un bool -> lin bool)"),
            Ok((
                "",
                PrimType::Arrow(
                    Box::new(TypeExpr {
                        qual: Qual::Un,
                        prim: PrimType::Bool
                    }),
                    Box::new(TypeExpr {
                        qual: Qual::Lin,
                        prim: PrimType::Bool
                    })
                )
            ))
        );
    }
}

fn type_expr<'a>() -> impl Parser<'a, TypeExpr> {
    qual()
        .join(prim_type())
        .map(|(q, p)| TypeExpr { qual: q, prim: p })
}
#[cfg(test)]
mod type_expr_test {
    use super::*;

    // #[test]
    fn test() {
        assert_eq!(
            type_expr().parse("un bool"),
            Ok((
                "",
                TypeExpr {
                    qual: Qual::Un,
                    prim: PrimType::Bool
                }
            ))
        );
    }
}
