//! BNF
//!
//! # 構文
//!
//! ```text
//! <VAR>   := [a-zA-Z_][a-zA-Z0-9_]*
//!
//! <E>     := <LET> | <IF> | <SPLIT> | <FREE> | <APP> | <VAR> | <QVAL>
//!
//! <LET>   := let <VAR> : <T> = <E>; <E>
//! <IF>    := if <E> { <E> } else { <E> }
//! <SPLIT> := split <E> as <VAR>, <VAR> { <E> }
//! <FREE>  := free <E>; <E>
//! <APP>   := ( <E> <E> )
//! <Q>     := lin | un
//!
//! 値
//! <QVAL>  := <Q> <VAL>
//! <VAL>   := <B> | <PAIR> | <FN>
//! <B>     := true | false
//! <PAIR>  := < <E> , <E> >
//! <FN>    := fn <VAR> : <T> { <E> }
//!
//! 型
//! <T>     := <Q> <P>
//! <P>     := bool | ( <T> * <T> ) | ( <T> -> <T> )
//! ```
use crate::lang::*;
use parser_combinator::*;

pub fn parse_expr(i: &str) -> ParseResult<Expr> {
    let (i, _) = space0().parse(i)?;
    let (next_i, tok) = first_token(i)?;

    match tok {
        "let" => parse_let(i),
        "if" => parse_if(i),
        "split" => parse_split(i),
        "free" => parse_free(i),
        "lin" | "un" => parse_qval(i),
        "(" => parse_app(i),
        _ => Ok((next_i, Expr::Var(tok.to_string()))),
    }
}
#[cfg(test)]
mod parse_expr {
    use super::*;

    #[test]
    fn test_parse_expr() {
        assert_eq!(
            parse_expr("let x: un bool = lin true; x"),
            Ok((
                "",
                Expr::Let(LetExpr {
                    var: "x".to_string(),
                    ty: TypeExpr {
                        qual: Qual::Un,
                        prim: PrimType::Bool
                    },
                    expr1: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Lin,
                        val: ValExpr::Bool(true)
                    })),
                    expr2: Box::new(Expr::Var("x".to_string())),
                })
            ))
        );
        assert_eq!(
            parse_expr("if lin true { lin false } else { lin true }"),
            Ok((
                "",
                Expr::If(IfExpr {
                    cond_expr: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Lin,
                        val: ValExpr::Bool(true)
                    })),
                    then_expr: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Lin,
                        val: ValExpr::Bool(false)
                    })),
                    else_expr: Box::new(Expr::QVal(QValExpr {
                        qual: Qual::Lin,
                        val: ValExpr::Bool(true)
                    })),
                })
            ))
        );
        assert_eq!(
            parse_expr("split v as x, y { x }"),
            Ok((
                "",
                Expr::Split(SplitExpr {
                    expr: Box::new(Expr::Var("v".to_string())),
                    left: "x".to_string(),
                    right: "y".to_string(),
                    body: Box::new(Expr::Var("x".to_string())),
                })
            ))
        );
        assert_eq!(
            parse_expr("free x; x"),
            Ok((
                "",
                Expr::Free(FreeExpr {
                    var: "x".to_string(),
                    expr: Box::new(Expr::Var("x".to_string())),
                })
            ))
        );
        assert_eq!(
            parse_expr("lin true"),
            Ok((
                "",
                Expr::QVal(QValExpr {
                    qual: Qual::Lin,
                    val: ValExpr::Bool(true)
                })
            ))
        );
        assert_eq!(
            parse_expr("un false"),
            Ok((
                "",
                Expr::QVal(QValExpr {
                    qual: Qual::Un,
                    val: ValExpr::Bool(false)
                })
            ))
        );
        assert_eq!(
            parse_expr("un <lin true, un false>"),
            Ok((
                "",
                Expr::QVal(QValExpr {
                    qual: Qual::Un,
                    val: ValExpr::Pair(
                        Box::new(Expr::QVal(QValExpr {
                            qual: Qual::Lin,
                            val: ValExpr::Bool(true)
                        })),
                        Box::new(Expr::QVal(QValExpr {
                            qual: Qual::Un,
                            val: ValExpr::Bool(false)
                        })),
                    )
                })
            ))
        );
        assert_eq!(parse_expr("abc"), Ok(("", Expr::Var("abc".to_string()))));
        assert_eq!(parse_expr("abc!"), Ok(("!", Expr::Var("abc".to_string()))));
    }
}

fn parse_var(input: &str) -> ParseResult<&str> {
    let mut pos = 0;
    let mut chars = input.chars();

    match chars.next() {
        Some(next) if next.is_alphabetic() || next == '_' => pos += 1,
        _ => return Err(input),
    }

    while let Some(next) = chars.next() {
        if next.is_alphanumeric() || next == '_' {
            pos += 1;
        } else {
            break;
        }
    }

    Ok((&input[pos..], &input[..pos]))
}
#[cfg(test)]
mod parse_var {
    use super::*;

    #[test]
    fn test_parse_var() {
        assert_eq!(parse_var("abc"), Ok(("", "abc")));
        assert_eq!(parse_var("abc123"), Ok(("", "abc123")));
        assert_eq!(parse_var("abc_123"), Ok(("", "abc_123")));
        assert_eq!(parse_var("abc_123def"), Ok(("", "abc_123def")));
        assert_eq!(parse_var("123abc"), Err("123abc"));
        assert_eq!(parse_var("123"), Err("123"));
        assert_eq!(parse_var("123abc"), Err("123abc"));
        assert_eq!(parse_var("abc!"), Ok(("!", "abc")));
    }
}

fn first_token(i: &str) -> ParseResult<&str> {
    match keyword("let")
        .or_else(keyword("if"))
        .or_else(keyword("split"))
        .or_else(keyword("free"))
        .or_else(keyword("lin"))
        .or_else(keyword("un"))
        .or_else(keyword("("))
        .parse(i)
    {
        ok @ Ok(_) => ok,
        Err(_) => parse_var(i),
    }
}
#[cfg(test)]
mod first_token {
    use super::*;

    #[test]
    fn test_first_token() {
        assert_eq!(first_token("let x y"), Ok((" x y", "let")));
        assert_eq!(
            first_token("if c { t } else { e }"),
            Ok((" c { t } else { e }", "if"))
        );
        assert_eq!(
            first_token("split v as x,y { e }"),
            Ok((" v as x,y { e }", "split"))
        );
        assert_eq!(first_token("free x; e"), Ok((" x; e", "free")));
        assert_eq!(first_token("lin true"), Ok((" true", "lin")));
        assert_eq!(first_token("un false"), Ok((" false", "un")));
        assert_eq!(
            first_token("(lin true, un false)"),
            Ok(("lin true, un false)", "("))
        );
        assert_eq!(first_token("abc"), Ok(("", "abc")));
        assert_eq!(first_token("abc!"), Ok(("!", "abc")));
    }
}

fn parse_let(i: &str) -> ParseResult<Expr> {
    let (i, _) = keyword("let").parse(i)?;
    let (i, _) = space1().parse(i)?;

    let (i, var) = parse_var(i)?;

    let (i, _) = space0().parse(i)?;
    let (i, _) = char(':').parse(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, ty) = parse_type(i)?;

    let (i, _) = space0().parse(i)?;
    let (i, _) = char('=').parse(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, e1) = parse_expr(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, _) = char(';').parse(i)?;
    let (i, e2) = parse_expr(i)?;

    Ok((
        i,
        Expr::Let(LetExpr {
            var: var.to_string(),
            ty,
            expr1: Box::new(e1),
            expr2: Box::new(e2),
        }),
    ))
}
#[cfg(test)]
mod parse_let {
    use super::*;

    #[test]
    fn test_parse_let() {
        assert_eq!(
            parse_let("let x : lin bool = e1; e2"),
            Ok((
                "",
                Expr::Let(LetExpr {
                    var: "x".to_string(),
                    ty: TypeExpr {
                        qual: Qual::Lin,
                        prim: PrimType::Bool
                    },
                    expr1: Box::new(Expr::Var("e1".to_string())),
                    expr2: Box::new(Expr::Var("e2".to_string())),
                })
            ))
        );
    }
}

fn parse_if(i: &str) -> ParseResult<Expr> {
    let (i, _) = keyword("if").parse(i)?;
    let (i, _) = space1().parse(i)?;

    let (i, e1) = parse_expr(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, e2) = braces(parse_expr).parse(i)?;

    let (i, _) = space0().parse(i)?;
    let (i, _) = keyword("else").parse(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, e3) = braces(parse_expr).parse(i)?;

    Ok((
        i,
        Expr::If(IfExpr {
            cond_expr: Box::new(e1),
            then_expr: Box::new(e2),
            else_expr: Box::new(e3),
        }),
    ))
}
#[cfg(test)]
mod parse_if {
    use super::*;

    #[test]
    fn test_parse_if() {
        assert_eq!(
            parse_if("if e1 { e2 } else { e3 }"),
            Ok((
                "",
                Expr::If(IfExpr {
                    cond_expr: Box::new(Expr::Var("e1".to_string())),
                    then_expr: Box::new(Expr::Var("e2".to_string())),
                    else_expr: Box::new(Expr::Var("e3".to_string())),
                })
            ))
        );
    }
}

fn parse_split(i: &str) -> ParseResult<Expr> {
    let (i, _) = keyword("split").parse(i)?;
    let (i, _) = space1().parse(i)?;

    let (i, e1) = parse_expr(i)?;

    let (i, _) = space1().parse(i)?;
    let (i, _) = keyword("as").parse(i)?;
    let (i, _) = space1().parse(i)?;

    let (i, var1) = parse_var(i)?;

    let (i, _) = space0().parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, var2) = parse_var(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, e2) = braces(parse_expr).parse(i)?;

    Ok((
        i,
        Expr::Split(SplitExpr {
            expr: Box::new(e1),
            left: var1.to_string(),
            right: var2.to_string(),
            body: Box::new(e2),
        }),
    ))
}
#[cfg(test)]
mod parse_split {
    use super::*;

    #[test]
    fn test_parse_split() {
        assert_eq!(
            parse_split("split e1 as x, y { e2 }"),
            Ok((
                "",
                Expr::Split(SplitExpr {
                    expr: Box::new(Expr::Var("e1".to_string())),
                    left: "x".to_string(),
                    right: "y".to_string(),
                    body: Box::new(Expr::Var("e2".to_string())),
                })
            ))
        );
    }
}

fn parse_free(i: &str) -> ParseResult<Expr> {
    let (i, _) = keyword("free").parse(i)?;
    let (i, _) = space1().parse(i)?;

    let (i, var) = parse_var(i)?;
    let (i, _) = space0().parse(i)?;
    let (i, _) = char(';').parse(i)?;

    let (i, e) = parse_expr(i)?;
    Ok((
        i,
        Expr::Free(FreeExpr {
            var: var.to_string(),
            expr: Box::new(e),
        }),
    ))
}
#[cfg(test)]
mod parse_free {
    use super::*;

    #[test]
    fn test_parse_free() {
        assert_eq!(
            parse_free("free x; e"),
            Ok((
                "",
                Expr::Free(FreeExpr {
                    var: "x".to_string(),
                    expr: Box::new(Expr::Var("e".to_string())),
                })
            ))
        );
    }
}

fn parse_qval(i: &str) -> ParseResult<Expr> {
    let (i, q) = parse_qual(i)?;
    let (i, _) = space1().parse(i)?;

    let (i, v) = parse_val(i)?;

    Ok((i, Expr::QVal(QValExpr { qual: q, val: v })))
}
#[cfg(test)]
mod parse_qval {
    use super::*;

    #[test]
    fn test_parse_qval() {
        assert_eq!(
            parse_qval("lin fn x : un bool { e }"),
            Ok((
                "",
                Expr::QVal(QValExpr {
                    qual: Qual::Lin,
                    val: ValExpr::Fun(FnExpr {
                        var: "x".to_string(),
                        ty: TypeExpr {
                            qual: Qual::Un,
                            prim: PrimType::Bool
                        },
                        expr: Box::new(Expr::Var("e".to_string())),
                    }),
                })
            ))
        );
    }
}

fn parse_val(i: &str) -> ParseResult<ValExpr> {
    let (next_i, tok) = keyword("fn")
        .or_else(keyword("true"))
        .or_else(keyword("false"))
        .or_else(keyword("<"))
        .parse(i)?;

    match tok {
        "fn" => parse_fn(i),
        "true" => Ok((next_i, ValExpr::Bool(true))),
        "false" => Ok((next_i, ValExpr::Bool(false))),
        "<" => parse_pair(i),
        _ => unreachable!(),
    }
}
#[cfg(test)]
mod parse_val {
    use super::*;

    #[test]
    fn test_parse_val() {
        assert_eq!(
            parse_val("fn x : un bool { e }"),
            Ok((
                "",
                ValExpr::Fun(FnExpr {
                    var: "x".to_string(),
                    ty: TypeExpr {
                        qual: Qual::Un,
                        prim: PrimType::Bool
                    },
                    expr: Box::new(Expr::Var("e".to_string())),
                })
            ))
        );
        assert_eq!(parse_val("true"), Ok(("", ValExpr::Bool(true))));
        assert_eq!(parse_val("false"), Ok(("", ValExpr::Bool(false))));
        assert_eq!(
            parse_val("<x, y>"),
            Ok((
                "",
                ValExpr::Pair(
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Var("y".to_string()))
                )
            ))
        );
    }
}

fn parse_fn(i: &str) -> ParseResult<ValExpr> {
    let (i, _) = keyword("fn").parse(i)?;
    let (i, _) = space1().parse(i)?;

    let (i, var) = parse_var(i)?;

    let (i, _) = space0().parse(i)?;
    let (i, _) = char(':').parse(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, ty) = parse_type(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, expr) = braces(parse_expr).parse(i)?;

    Ok((
        i,
        ValExpr::Fun(FnExpr {
            var: var.to_string(),
            ty,
            expr: Box::new(expr),
        }),
    ))
}
#[cfg(test)]
mod parse_fn {
    use super::*;

    #[test]
    fn test_parse_fn() {
        assert_eq!(
            parse_fn("fn x : un bool { e }"),
            Ok((
                "",
                ValExpr::Fun(FnExpr {
                    var: "x".to_string(),
                    ty: TypeExpr {
                        qual: Qual::Un,
                        prim: PrimType::Bool
                    },
                    expr: Box::new(Expr::Var("e".to_string())),
                })
            ))
        );
    }
}

fn parse_pair(i: &str) -> ParseResult<ValExpr> {
    let (i, _) = char('<').parse(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, e1) = parse_expr(i)?;

    let (i, _) = space0().parse(i)?;
    let (i, _) = char(',').parse(i)?;
    let (i, _) = space0().parse(i)?;

    let (i, e2) = parse_expr(i)?;

    let (i, _) = space0().parse(i)?;
    let (i, _) = char('>').parse(i)?;

    Ok((i, ValExpr::Pair(Box::new(e1), Box::new(e2))))
}
#[cfg(test)]
mod parse_pair {
    use super::*;

    #[test]
    fn test_parse_pair() {
        assert_eq!(
            parse_pair("<x, y>"),
            Ok((
                "",
                ValExpr::Pair(
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Var("y".to_string()))
                )
            ))
        );
    }
}

fn parse_app(i: &str) -> ParseResult<Expr> {
    let (i, _) = char('(').parse(i)?;
    let (i, _) = space0().parse(i)?;
    let (i, e1) = parse_expr(i)?;

    let (i, _) = space1().parse(i)?;

    let (i, e2) = parse_expr(i)?;

    let (i, _) = space0().parse(i)?;
    let (i, _) = char(')').parse(i)?;

    Ok((
        i,
        Expr::App(AppExpr {
            expr1: Box::new(e1),
            expr2: Box::new(e2),
        }),
    ))
}
#[cfg(test)]
mod parse_app {
    use super::*;

    #[test]
    fn test_parse_app() {
        assert_eq!(
            parse_app("(e1 e2)"),
            Ok((
                "",
                Expr::App(AppExpr {
                    expr1: Box::new(Expr::Var("e1".to_string())),
                    expr2: Box::new(Expr::Var("e2".to_string())),
                })
            ))
        );
    }
}

fn parse_type(i: &str) -> ParseResult<TypeExpr> {
    let (i, qual) = parse_qual(i)?;
    let (i, _) = space1().parse(i)?;
    let (i, val) = keyword("bool").or_else(keyword("(")).parse(i)?;
    if val == "bool" {
        Ok((
            i,
            TypeExpr {
                qual,
                prim: PrimType::Bool,
            },
        ))
    } else {
        let (i, _) = space0().parse(i)?;
        let (i, t1) = parse_type(i)?;
        let (i, _) = space0().parse(i)?;

        let (i, op) = keyword("*").or_else(keyword("->")).parse(i)?;

        let (i, _) = space0().parse(i)?;
        let (i, t2) = parse_type(i)?;
        let (i, _) = space0().parse(i)?;

        let (i, _) = char(')').parse(i)?;

        Ok((
            i,
            TypeExpr {
                qual,
                prim: match op {
                    "*" => PrimType::Pair(Box::new(t1), Box::new(t2)),
                    "->" => PrimType::Arrow(Box::new(t1), Box::new(t2)),
                    _ => unreachable!(),
                },
            },
        ))
    }
}
#[cfg(test)]
mod parse_type {
    use super::*;

    #[test]
    fn test_parse_type() {
        assert_eq!(
            parse_type("lin bool"),
            Ok((
                "",
                TypeExpr {
                    qual: Qual::Lin,
                    prim: PrimType::Bool
                }
            ))
        );
        assert_eq!(
            parse_type("un bool"),
            Ok((
                "",
                TypeExpr {
                    qual: Qual::Un,
                    prim: PrimType::Bool
                }
            ))
        );
        assert_eq!(
            parse_type("lin (un bool * un bool)"),
            Ok((
                "",
                TypeExpr {
                    qual: Qual::Lin,
                    prim: PrimType::Pair(
                        Box::new(TypeExpr {
                            qual: Qual::Un,
                            prim: PrimType::Bool
                        }),
                        Box::new(TypeExpr {
                            qual: Qual::Un,
                            prim: PrimType::Bool
                        })
                    )
                }
            ))
        );
        assert_eq!(
            parse_type("un (lin bool -> un bool)"),
            Ok((
                "",
                TypeExpr {
                    qual: Qual::Un,
                    prim: PrimType::Arrow(
                        Box::new(TypeExpr {
                            qual: Qual::Lin,
                            prim: PrimType::Bool
                        }),
                        Box::new(TypeExpr {
                            qual: Qual::Un,
                            prim: PrimType::Bool
                        })
                    )
                }
            ))
        );
    }
}

fn parse_qual(i: &str) -> ParseResult<Qual> {
    let (i, q) = keyword("lin").or_else(keyword("un")).parse(i)?;
    match q {
        "lin" => Ok((i, Qual::Lin)),
        "un" => Ok((i, Qual::Un)),
        _ => unreachable!(),
    }
}
#[cfg(test)]
mod parse_qual {
    use super::*;

    #[test]
    fn test_parse_qual() {
        assert_eq!(parse_qual("lin"), Ok(("", Qual::Lin)));
        assert_eq!(parse_qual("un"), Ok(("", Qual::Un)));
    }
}
