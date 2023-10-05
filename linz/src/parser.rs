//! BNF
//!
//! <Q> := lin | un
//! <B> := true | false
//! <V> := [a-zA-Z_][a-zA-Z0-9_]+
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
//!      | <Y>, <V> : <T>
use crate::lang::*;
use parser_combinator::*;

/// tokenize program code
fn tokenize(line: &str) -> Vec<(usize, String)> {
    use std::mem::take;

    let len = line.len();
    let mut result = vec![];
    let mut chars = line.chars().peekable();
    let mut token = String::new();

    while let Some(c) = chars.next() {
        match c {
            // 空白読み飛ばし
            c if c.is_whitespace() => {
                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }
            }
            // -> は 2 文字トークン
            c if c == '-' => {
                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }
                if let Some(&next_c) = chars.peek() {
                    if next_c == '>' {
                        chars.next();
                        let cc = String::from_utf8(vec![c as u8, next_c as u8]).unwrap();
                        result.push((len - chars.clone().count() - 2, cc));
                        continue;
                    }
                }

                result.push((len - chars.clone().count() - 1, c.to_string()));
            }
            // これらは 1 文字トークン
            '{' | '}' | '(' | ')' | '<' | '>' | ':' | ';' | ',' | '*' => {
                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }
                result.push((len - chars.clone().count() - 1, c.to_string()));
            }
            _ => token.push(c),
        }
    }
    if token.len() > 0 {
        result.push((len - chars.clone().count() - token.len(), token.to_string()));
    }
    result
}

#[cfg(test)]
mod tokenize {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(tokenize(""), vec![]);
        assert_eq!(tokenize("foo"), vec![(0, "foo".to_string())]);
        assert_eq!(tokenize("42"), vec![(0, "42".to_string())]);
        assert_eq!(
            tokenize("lin fn x : lin bool {if x {lin false} else {lin true}}"),
            vec![
                (0, "lin".to_string()),
                (4, "fn".to_string()),
                (7, "x".to_string()),
                (9, ":".to_string()),
                (11, "lin".to_string()),
                (15, "bool".to_string()),
                (20, "{".to_string()),
                (21, "if".to_string()),
                (24, "x".to_string()),
                (26, "{".to_string()),
                (27, "lin".to_string()),
                (31, "false".to_string()),
                (36, "}".to_string()),
                (38, "else".to_string()),
                (43, "{".to_string()),
                (44, "lin".to_string()),
                (48, "true".to_string()),
                (52, "}".to_string()),
                (53, "}".to_string())
            ]
        );
        assert_eq!(
            tokenize("un fn x : lin(lin bool * lin bool){split x as a,b {lin <b, a>}}"),
            vec![
                (0, "un".to_string()),
                (3, "fn".to_string()),
                (6, "x".to_string()),
                (8, ":".to_string()),
                (10, "lin".to_string()),
                (13, "(".to_string()),
                (14, "lin".to_string()),
                (18, "bool".to_string()),
                (23, "*".to_string()),
                (25, "lin".to_string()),
                (29, "bool".to_string()),
                (33, ")".to_string()),
                (34, "{".to_string()),
                (35, "split".to_string()),
                (41, "x".to_string()),
                (43, "as".to_string()),
                (46, "a".to_string()),
                (47, ",".to_string()),
                (48, "b".to_string()),
                (50, "{".to_string()),
                (51, "lin".to_string()),
                (55, "<".to_string()),
                (56, "b".to_string()),
                (57, ",".to_string()),
                (59, "a".to_string()),
                (60, ">".to_string()),
                (61, "}".to_string()),
                (62, "}".to_string())
            ]
        );
        assert_eq!(
            tokenize("lin fn x : lin (lin bool * lin bool) {split x as a, b {free b; a}}"),
            vec![
                (0, "lin".to_string()),
                (4, "fn".to_string()),
                (7, "x".to_string()),
                (9, ":".to_string()),
                (11, "lin".to_string()),
                (15, "(".to_string()),
                (16, "lin".to_string()),
                (20, "bool".to_string()),
                (25, "*".to_string()),
                (27, "lin".to_string()),
                (31, "bool".to_string()),
                (35, ")".to_string()),
                (37, "{".to_string()),
                (38, "split".to_string()),
                (44, "x".to_string()),
                (46, "as".to_string()),
                (49, "a".to_string()),
                (50, ",".to_string()),
                (52, "b".to_string()),
                (54, "{".to_string()),
                (55, "free".to_string()),
                (60, "b".to_string()),
                (61, ";".to_string()),
                (63, "a".to_string()),
                (64, "}".to_string()),
                (65, "}".to_string())
            ]
        );
    }
}

fn qual() -> impl Parser<Output = Qual> {
    let lin = apply(literal("lin"), |_| Qual::Lin);
    let un = apply(literal("un"), |_| Qual::Un);
    altl(lin, un)
}

fn var() -> impl Parser<Output = String> {
    satisfy(|s| {
        let cs = s.chars().collect::<Vec<_>>();
        cs.len() > 0
            && (cs[0].is_alphabetic() || cs[0] == '_')
            && cs[1..].iter().all(|c| c.is_alphanumeric() || *c == '_')
    })
}
