//! Parser combinators.
//!
//! # Examples
//!
//! ```
//! use parser_combinators::*;
//! let p = bind(literal("foo"), |s| empty(s.len()));
//! assert_eq!(
//!     p.parse(vec![(0, "foo".to_string())].into()),
//!     vec![(3, vec![].into())]
//! );
//! assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
//! ```
use std::collections::VecDeque;

pub type Location = usize;
pub type Token = (Location, String);

pub trait Parser {
    type Output;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)>;
}

#[derive(Debug, Clone)]
pub struct Sat<F: Fn(String) -> bool> {
    pred: F,
}
impl<F> Parser for Sat<F>
where
    F: Fn(String) -> bool,
{
    type Output = String;

    fn parse(&self, mut tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        if let Some((_, token)) = tokens.pop_front() {
            if (self.pred)(token.clone()) {
                return vec![(token, tokens)];
            }
        }
        vec![]
    }
}
/// Satisfies the given predicate.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = satisfy(|s| s == "foo");
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string())].into()),
///     vec![("foo".to_string(), vec![].into())]
/// );
/// assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
/// ```
#[allow(dead_code)]
pub fn satisfy<F: Fn(String) -> bool>(pred: F) -> Sat<F> {
    Sat { pred }
}
#[cfg(test)]
mod sat {
    #[test]
    fn test() {
        use super::*;
        let p = satisfy(|s| s == "foo");
        assert_eq!(
            p.parse(vec![(0, "foo".to_string())].into()),
            vec![("foo".to_string(), vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct Lit {
    s: &'static str,
}
impl Parser for Lit {
    type Output = String;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        let s = self.s;
        Sat {
            pred: Box::new(move |token| token == s),
        }
        .parse(tokens)
    }
}
/// Parses the given literal.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = literal("foo");
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string())].into()),
///     vec![("foo".to_string(), vec![].into())]
/// );
/// assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
/// ```
#[allow(dead_code)]
pub fn literal(s: &'static str) -> Lit {
    Lit { s }
}
#[cfg(test)]
mod lit {
    #[test]
    fn test() {
        use super::*;
        let p = literal("foo");
        assert_eq!(
            p.parse(vec![(0, "foo".to_string())].into()),
            vec![("foo".to_string(), vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct Empty<T: Clone>(T);
impl<T: Clone> Parser for Empty<T> {
    type Output = T;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        vec![(self.0.clone(), tokens)]
    }
}
/// Always succeeds with the given value.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = empty(42);
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string())].into()),
///     vec![(42, vec![(0, "foo".to_string())].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn empty<T: Clone>(x: T) -> Empty<T> {
    Empty(x)
}
#[cfg(test)]
mod empty {
    #[test]
    fn test() {
        use super::*;
        let p = empty(42);
        assert_eq!(
            p.parse(vec![(0, "foo".to_string())].into()),
            vec![(42, vec![(0, "foo".to_string())].into())]
        );
    }
}

#[derive(Debug, Clone)]
pub struct Bind<T, P: Parser<Output = T>, F: Fn(T) -> Q, Q: Parser> {
    px: P,
    f: F,
}
impl<T, P: Parser<Output = T>, F: Fn(T) -> Q, Q: Parser> Parser for Bind<T, P, F, Q> {
    type Output = Q::Output;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        let mut result = vec![];
        for (x, tokens) in self.px.parse(tokens) {
            for (y, tokens) in (self.f)(x).parse(tokens) {
                result.push((y, tokens));
            }
        }
        result
    }
}
/// Binds the result of the given parser to the given function.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = bind(literal("foo"), |s| empty(s.len()));
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string())].into()),
///     vec![(3, vec![].into())]
/// );
/// assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
/// ```
#[allow(dead_code)]
pub fn bind<T, P: Parser<Output = T>, F: Fn(T) -> Q, Q: Parser>(px: P, f: F) -> Bind<T, P, F, Q> {
    Bind { px, f }
}
#[cfg(test)]
mod bind {
    #[test]
    fn test() {
        use super::*;
        let p = bind(literal("foo"), |s| empty(s.len()));
        assert_eq!(
            p.parse(vec![(0, "foo".to_string())].into()),
            vec![(3, vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct Apply<T, U, P: Parser<Output = T>, F: Fn(T) -> U> {
    px: P,
    f: F,
}
impl<T: Clone, U, P: Parser<Output = T>, F: Fn(T) -> U> Parser for Apply<T, U, P, F> {
    type Output = U;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        let mut result = vec![];
        for (x, tokens) in self.px.parse(tokens) {
            result.push(((self.f)(x.clone()), tokens));
        }
        result
    }
}
/// Applies the given function to the result of the given parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = apply(int32(), |n| n * 2);
/// assert_eq!(
///     p.parse(vec![(0, "21".to_string())].into()),
///     vec![(42, vec![].into())]
/// );
/// assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
/// ```
#[allow(dead_code)]
pub fn apply<T, U, P: Parser<Output = T>, F: Fn(T) -> U>(px: P, f: F) -> Apply<T, U, P, F> {
    Apply { px, f }
}
#[cfg(test)]
mod apply {
    #[test]
    fn test() {
        use super::*;
        let p = apply(literal("foo"), |s| s.len());
        assert_eq!(
            p.parse(vec![(0, "foo".to_string())].into()),
            vec![(3, vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
        let p = apply(int32(), |n| n * 2);
        assert_eq!(
            p.parse(vec![(0, "21".to_string())].into()),
            vec![(42, vec![].into())]
        );
    }
}

#[derive(Debug, Clone)]
pub struct Apply2<T, U, V, P: Parser<Output = T>, Q: Parser<Output = U>> {
    px: P,
    qx: Q,
    f: fn(T, U) -> V,
}
impl<T: Clone, U, V, P: Parser<Output = T>, Q: Parser<Output = U>> Parser
    for Apply2<T, U, V, P, Q>
{
    type Output = V;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        let mut result = vec![];
        for (x, tokens) in self.px.parse(tokens.clone()) {
            for (y, tokens) in self.qx.parse(tokens) {
                result.push(((self.f)(x.clone(), y), tokens));
            }
        }
        result
    }
}
/// Applies the given function to the results of the given parsers.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = apply2(literal("foo"), literal("bar"), |s, t| (s.len(), t.len()));
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
///     vec![((3, 3), vec![].into())]
/// );
/// assert_eq!(p.parse(vec![(0, "foobar".to_string())].into()), vec![]);
/// ```
#[allow(dead_code)]
pub fn apply2<T, U, V, P: Parser<Output = T>, Q: Parser<Output = U>>(
    px: P,
    qx: Q,
    f: fn(T, U) -> V,
) -> Apply2<T, U, V, P, Q> {
    Apply2 { px, qx, f }
}
#[cfg(test)]
mod apply2 {
    #[test]
    fn test() {
        use super::*;
        let p = apply2(literal("foo"), literal("bar"), |s, t| (s.len(), t.len()));
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
            vec![((3, 3), vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "foobar".to_string())].into()), vec![]);
        assert_eq!(p.parse(vec![(0, "foo".to_string())].into()), vec![]);
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "buz".to_string())].into()),
            vec![]
        );
    }
}

#[derive(Debug, Clone)]
pub struct Alt<T, P: Parser<Output = T>, Q: Parser<Output = T>> {
    px: P,
    qx: Q,
}
impl<T, P: Parser<Output = T>, Q: Parser<Output = T>> Parser for Alt<T, P, Q> {
    type Output = T;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        let mut result = vec![];
        for (x, tokens) in self.px.parse(tokens.clone()) {
            result.push((x, tokens));
        }
        for (x, tokens) in self.qx.parse(tokens) {
            result.push((x, tokens));
        }
        result
    }
}
/// Applies the given parsers in order and returns the both results.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = alt(
///     literal("foo"),
///     apply2(literal("foo"), literal("bar"), |s, t| s + &t),
/// );
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
///     vec![
///         ("foo".to_string(), vec![(1, "bar".to_string())].into()),
///         ("foobar".to_string(), vec![].into())
///     ]
/// );
/// ```
#[allow(dead_code)]
pub fn alt<T, P: Parser<Output = T>, Q: Parser<Output = T>>(px: P, qx: Q) -> Alt<T, P, Q> {
    Alt { px, qx }
}
#[cfg(test)]
mod alt {
    #[test]
    fn basic() {
        use super::*;
        let p = alt(literal("foo"), literal("bar"));
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
            vec![("foo".to_string(), vec![(1, "bar".to_string())].into())]
        );
        assert_eq!(
            p.parse(vec![(0, "foo".to_string())].into()),
            vec![("foo".to_string(), vec![].into())]
        );
        assert_eq!(
            p.parse(vec![(0, "bar".to_string())].into()),
            vec![("bar".to_string(), vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "baz".to_string())].into()), vec![]);
    }
    #[test]
    fn both() {
        use super::*;
        let p = alt(
            literal("foo"),
            apply2(literal("foo"), literal("bar"), |s, t| s + &t),
        );
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
            vec![
                ("foo".to_string(), vec![(1, "bar".to_string())].into()),
                ("foobar".to_string(), vec![].into())
            ]
        );
    }
}

#[derive(Debug, Clone)]
pub struct AltL<T, P: Parser<Output = T>, Q: Parser<Output = T>> {
    px: P,
    qx: Q,
}
impl<T, P: Parser<Output = T>, Q: Parser<Output = T>> Parser for AltL<T, P, Q> {
    type Output = T;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        let mut result = vec![];
        for (x, tokens) in self.px.parse(tokens.clone()) {
            result.push((x, tokens));
        }
        if result.is_empty() {
            for (x, tokens) in self.qx.parse(tokens) {
                result.push((x, tokens));
            }
        }
        result
    }
}
/// Alternative combinator that returns the result of the first parser if it succeeds.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = altl(
///     literal("foo"),
///     apply2(literal("foo"), literal("bar"), |s, t| s + &t),
/// );
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
///     vec![
///         ("foo".to_string(), vec![(1, "bar".to_string())].into()),
///     ]
/// );
/// ```
#[allow(dead_code)]
pub fn altl<T, P: Parser<Output = T>, Q: Parser<Output = T>>(px: P, qx: Q) -> AltL<T, P, Q> {
    AltL { px, qx }
}
#[cfg(test)]
mod altl {
    #[test]
    fn basic() {
        use super::*;
        let p = altl(literal("foo"), literal("bar"));
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
            vec![("foo".to_string(), vec![(1, "bar".to_string())].into())]
        );
        assert_eq!(
            p.parse(vec![(0, "foo".to_string())].into()),
            vec![("foo".to_string(), vec![].into())]
        );
        assert_eq!(
            p.parse(vec![(0, "bar".to_string())].into()),
            vec![("bar".to_string(), vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "baz".to_string())].into()), vec![]);
    }
    #[test]
    fn both() {
        use super::*;
        let p = altl(
            literal("foo"),
            apply2(literal("foo"), literal("bar"), |s, t| s + &t),
        );
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
            vec![("foo".to_string(), vec![(1, "bar".to_string())].into())]
        );
    }
}

#[derive(Debug, Clone)]
pub struct Ap<T, U, F: Fn(&T) -> U, P: Parser<Output = T>, Q: Parser<Output = F>> {
    px: P,
    pf: Q,
}
impl<T, U, F: Fn(&T) -> U, P: Parser<Output = T>, Q: Parser<Output = F>> Parser
    for Ap<T, U, F, P, Q>
{
    type Output = U;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        let mut result = vec![];
        for (x, tokens) in self.px.parse(tokens.clone()) {
            for (f, tokens) in self.pf.parse(tokens) {
                result.push(((f)(&x), tokens));
            }
        }
        result
    }
}
/// Applicative combinator that applies a function to the result of a parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = ap(
///     literal("hello"),
///     empty(|x: &String| x.to_string() + "world"),
/// );
/// assert_eq!(
///     p.parse(vec![(0, "hello".to_string()), (1, "foo".to_string())].into()),
///     vec![(
///         "helloworld".to_string(),
///         vec![(1, "foo".to_string())].into()
///     )]
/// );
/// ```
#[allow(dead_code)]
pub fn ap<T, U, F: Fn(&T) -> U, P: Parser<Output = T>, Q: Parser<Output = F>>(
    px: P,
    pf: Q,
) -> Ap<T, U, F, P, Q> {
    Ap { px, pf }
}
#[cfg(test)]
mod ap {
    #[test]
    fn test() {
        use super::*;
        let p = ap(
            literal("hello"),
            empty(|x: &String| x.to_string() + "world"),
        );
        assert_eq!(
            p.parse(vec![(0, "hello".to_string()), (1, "foo".to_string())].into()),
            vec![(
                "helloworld".to_string(),
                vec![(1, "foo".to_string())].into()
            )]
        );
        assert_eq!(p.parse(vec![(0, "foo".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct OneOrMore<T: Clone, P: Parser<Output = T>> {
    p: P,
}
impl<T: Clone, P: Parser<Output = T> + Clone> Parser for OneOrMore<T, P> {
    type Output = Vec<T>;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        Apply2 {
            px: self.p.clone(),
            qx: ZeroOrMore { p: self.p.clone() },
            f: |x, mut xs| {
                xs.insert(0, x.clone());
                xs
            },
        }
        .parse(tokens)
    }
}
/// Parser that matches one or more instances of a parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = one_or_more(literal("foo"));
/// assert_eq!(
///     p.parse(
///         vec![
///             (0, "foo".to_string()),
///             (1, "foo".to_string()),
///             (2, "foo".to_string())
///         ]
///         .into()
///     ),
///     vec![
///         (
///             vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
///             vec![].into()
///         ),
///         (
///             vec!["foo".to_string(), "foo".to_string()],
///             vec![(2, "foo".to_string())].into()
///         ),
///         (
///             vec!["foo".to_string()],
///             vec![(1, "foo".to_string()), (2, "foo".to_string())].into()
///         ),
///     ]
/// );
/// ```
#[allow(dead_code)]
pub fn one_or_more<T: Clone, P: Parser<Output = T> + Clone>(p: P) -> OneOrMore<T, P> {
    OneOrMore { p }
}
#[cfg(test)]
mod one_or_more {
    #[test]
    fn test() {
        use super::*;
        let p = one_or_more(literal("foo"));
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "foo".to_string()),
                    (2, "foo".to_string())
                ]
                .into()
            ),
            vec![
                (
                    vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
                    vec![].into()
                ),
                (
                    vec!["foo".to_string(), "foo".to_string()],
                    vec![(2, "foo".to_string())].into()
                ),
                (
                    vec!["foo".to_string()],
                    vec![(1, "foo".to_string()), (2, "foo".to_string())].into()
                )
            ]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "foo".to_string()),
                    (2, "bar".to_string())
                ]
                .into()
            ),
            vec![
                (
                    vec!["foo".to_string(), "foo".to_string()],
                    vec![(2, "bar".to_string())].into()
                ),
                (
                    vec!["foo".to_string()],
                    vec![(1, "foo".to_string()), (2, "bar".to_string())].into()
                )
            ]
        );
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct ZeroOrMore<T: Clone, P: Parser<Output = T>> {
    p: P,
}
impl<T: Clone, P: Parser<Output = T> + Clone> Parser for ZeroOrMore<T, P> {
    type Output = Vec<T>;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        Alt {
            px: OneOrMore { p: self.p.clone() },
            qx: Empty(vec![]),
        }
        .parse(tokens)
    }
}
/// Parser that matches zero or more instances of a parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = zero_or_more(literal("foo"));
/// assert_eq!(
///     p.parse(
///         vec![
///             (0, "foo".to_string()),
///             (1, "foo".to_string()),
///             (2, "foo".to_string())
///         ]
///         .into()
///     ),
///     vec![
///         (
///             vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
///             vec![].into()
///         ),
///         (
///             vec!["foo".to_string(), "foo".to_string()],
///             vec![(2, "foo".to_string())].into()
///         ),
///         (
///             vec!["foo".to_string()],
///             vec![(1, "foo".to_string()), (2, "foo".to_string())].into()
///         ),
///         (vec![], vec![(0, "foo".to_string()), (1, "foo".to_string()), (2, "foo".to_string())].into()),
///     ]
/// );
/// ```
#[allow(dead_code)]
pub fn zero_or_more<T: Clone, P: Parser<Output = T> + Clone>(p: P) -> ZeroOrMore<T, P> {
    ZeroOrMore { p }
}
#[cfg(test)]
mod zero_or_more {
    #[test]
    fn test() {
        use super::*;
        let p = zero_or_more(literal("foo"));
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "foo".to_string()),
                    (2, "foo".to_string())
                ]
                .into()
            ),
            vec![
                (
                    vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
                    vec![].into()
                ),
                (
                    vec!["foo".to_string(), "foo".to_string()],
                    vec![(2, "foo".to_string())].into()
                ),
                (
                    vec!["foo".to_string()],
                    vec![(1, "foo".to_string()), (2, "foo".to_string())].into()
                ),
                (
                    vec![],
                    vec![
                        (0, "foo".to_string()),
                        (1, "foo".to_string()),
                        (2, "foo".to_string())
                    ]
                    .into()
                )
            ]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "foo".to_string()),
                    (2, "bar".to_string())
                ]
                .into()
            ),
            vec![
                (
                    vec!["foo".to_string(), "foo".to_string()],
                    vec![(2, "bar".to_string())].into()
                ),
                (
                    vec!["foo".to_string()],
                    vec![(1, "foo".to_string()), (2, "bar".to_string())].into()
                ),
                (
                    vec![],
                    vec![
                        (0, "foo".to_string()),
                        (1, "foo".to_string()),
                        (2, "bar".to_string())
                    ]
                    .into()
                )
            ]
        );
        assert_eq!(
            p.parse(vec![(0, "bar".to_string())].into()),
            vec![(vec![], vec![(0, "bar".to_string())].into())]
        );
    }
}

#[derive(Debug, Clone)]
pub struct Munch1<T: Clone, P: Parser<Output = T>> {
    px: P,
}
impl<T: Clone, P: Parser<Output = T> + Clone> Parser for Munch1<T, P> {
    type Output = Vec<T>;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        Apply2 {
            px: self.px.clone(),
            qx: Munch {
                px: self.px.clone(),
            },
            f: |x, mut xs| {
                xs.insert(0, x.clone());
                xs
            },
        }
        .parse(tokens)
    }
}
/// Parser that matches one or more instances of a parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = munch1(literal("foo"));
/// assert_eq!(
///     p.parse(
///         vec![
///             (0, "foo".to_string()),
///             (1, "foo".to_string()),
///             (2, "foo".to_string())
///         ]
///         .into()
///     ),
///     vec![(
///         vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
///         vec![].into()
///     )]
/// );
/// ```
#[allow(dead_code)]
pub fn munch1<T: Clone, P: Parser<Output = T> + Clone>(p: P) -> Munch1<T, P> {
    Munch1 { px: p }
}
#[cfg(test)]
mod munch1 {
    #[test]
    fn test() {
        use super::*;
        let p = munch1(literal("foo"));
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "foo".to_string()),
                    (2, "foo".to_string())
                ]
                .into()
            ),
            vec![(
                vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
                vec![].into()
            )]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "foo".to_string()),
                    (2, "bar".to_string())
                ]
                .into()
            ),
            vec![(
                vec!["foo".to_string(), "foo".to_string()],
                vec![(2, "bar".to_string())].into()
            )]
        );
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct Munch<T: Clone, P: Parser<Output = T>> {
    px: P,
}
impl<T: Clone, P: Parser<Output = T> + Clone> Parser for Munch<T, P> {
    type Output = Vec<T>;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        AltL {
            px: Munch1 {
                px: self.px.clone(),
            },
            qx: Empty(vec![]),
        }
        .parse(tokens)
    }
}
/// Parser that matches zero or more instances of a parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = munch(literal("foo"));
/// assert_eq!(
///     p.parse(
///         vec![
///             (0, "foo".to_string()),
///             (1, "foo".to_string()),
///             (2, "foo".to_string())
///         ]
///         .into()
///     ),
///     vec![(
///         vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
///         vec![].into()
///     )]
/// );
/// ```
#[allow(dead_code)]
pub fn munch<T: Clone, P: Parser<Output = T> + Clone>(p: P) -> Munch<T, P> {
    Munch { px: p }
}
#[cfg(test)]
mod munch {
    #[test]
    fn test() {
        use super::*;
        let p = munch(literal("foo"));
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "foo".to_string()),
                    (2, "foo".to_string())
                ]
                .into()
            ),
            vec![(
                vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
                vec![].into()
            )]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "foo".to_string()),
                    (2, "bar".to_string())
                ]
                .into()
            ),
            vec![(
                vec!["foo".to_string(), "foo".to_string()],
                vec![(2, "bar".to_string())].into()
            )]
        );
        assert_eq!(
            p.parse(vec![(0, "bar".to_string())].into()),
            vec![(vec![], vec![(0, "bar".to_string())].into())]
        );
    }
}

#[derive(Debug, Clone)]
pub struct With<T, P: Parser<Output = T>, Q: Parser> {
    p: P,
    with: Q,
}
impl<T: Clone, P: Parser<Output = T> + Clone, Q: Parser + Clone> Parser for With<T, P, Q> {
    type Output = T;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        Bind {
            px: self.p.clone(),
            f: |x| Bind {
                px: self.with.clone(),
                f: move |_| Empty(x.clone()),
            },
        }
        .parse(tokens)
    }
}
/// Parser that matches a parser and then another parser, returning the result of the first parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = with(literal("foo"), literal("bar"));
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
///     vec![("foo".to_string(), vec![].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn with<T: Clone, P: Parser<Output = T> + Clone, Q: Parser + Clone>(
    p: P,
    with: Q,
) -> With<T, P, Q> {
    With { p, with }
}
#[cfg(test)]
mod with {
    #[test]
    fn test() {
        use super::*;
        let p = with(literal("foo"), literal("bar"));
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
            vec![("foo".to_string(), vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "foo".to_string())].into()), vec![]);
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct Skip<T, Q: Parser, P: Parser<Output = T>> {
    skip: Q,
    p: P,
}
impl<T: Clone, Q: Parser + Clone, P: Parser<Output = T> + Clone> Parser for Skip<T, Q, P> {
    type Output = T;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        Bind {
            px: self.skip.clone(),
            f: |_| self.p.clone(),
        }
        .parse(tokens)
    }
}
/// Parser that matches a parser and then another parser, returning the result of the second parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = skip(literal("foo"), literal("bar"));
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
///     vec![("bar".to_string(), vec![].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn skip<T: Clone, Q: Parser + Clone, P: Parser<Output = T> + Clone>(
    skip: Q,
    p: P,
) -> Skip<T, Q, P> {
    Skip { skip, p }
}
#[cfg(test)]
mod skip {
    #[test]
    fn test() {
        use super::*;
        let p = skip(literal("foo"), literal("bar"));
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
            vec![("bar".to_string(), vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "foo".to_string())].into()), vec![]);
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct OneOrMoreWithSep<T, P: Parser<Output = T>, Q: Parser> {
    p: P,
    sep: Q,
}
impl<T: Clone, P: Parser<Output = T> + Clone, Q: Parser + Clone> Parser
    for OneOrMoreWithSep<T, P, Q>
{
    type Output = Vec<T>;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        Apply2 {
            px: self.p.clone(),
            qx: ZeroOrMore {
                p: Skip {
                    skip: self.sep.clone(),
                    p: self.p.clone(),
                },
            },
            f: |x, mut xs| {
                xs.insert(0, x.clone());
                xs
            },
        }
        .parse(tokens)
    }
}
/// Parser that matches a parser one or more times, separated by another parser, returning a vector of the results of the first parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = one_or_more_with_sep(literal("foo"), literal(","));
/// assert_eq!(
///     p.parse(
///         vec![
///             (0, "foo".to_string()),
///             (1, ",".to_string()),
///             (2, "foo".to_string()),
///             (3, ",".to_string()),
///             (4, "foo".to_string())
///         ]
///         .into()
///     ),
///     vec![
///         (
///             vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
///             vec![].into()
///         )
///     ]
/// );
/// ```
#[allow(dead_code)]
pub fn one_or_more_with_sep<T: Clone, P: Parser<Output = T> + Clone, Q: Parser + Clone>(
    p: P,
    sep: Q,
) -> OneOrMoreWithSep<T, P, Q> {
    OneOrMoreWithSep { p, sep }
}
#[cfg(test)]
mod one_or_more_with_sep {
    #[test]
    fn test() {
        use super::*;
        let p = one_or_more_with_sep(literal("foo"), literal(","));
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, ",".to_string()),
                    (2, "foo".to_string()),
                    (3, ",".to_string()),
                    (4, "foo".to_string())
                ]
                .into()
            ),
            vec![
                (
                    vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
                    vec![].into()
                ),
                (
                    vec!["foo".to_string(), "foo".to_string()],
                    vec![(3, ",".to_string()), (4, "foo".to_string())].into()
                ),
                (
                    vec!["foo".to_string()],
                    vec![
                        (1, ",".to_string()),
                        (2, "foo".to_string()),
                        (3, ",".to_string()),
                        (4, "foo".to_string())
                    ]
                    .into()
                )
            ]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, ",".to_string()),
                    (2, "foo".to_string()),
                    (3, ",".to_string()),
                    (4, "bar".to_string())
                ]
                .into()
            ),
            vec![
                (
                    vec!["foo".to_string(), "foo".to_string()],
                    vec![(3, ",".to_string()), (4, "bar".to_string())].into()
                ),
                (
                    vec!["foo".to_string()],
                    vec![
                        (1, ",".to_string()),
                        (2, "foo".to_string()),
                        (3, ",".to_string()),
                        (4, "bar".to_string())
                    ]
                    .into()
                )
            ]
        );
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string()),].into()),
            vec![(vec!["foo".to_string()], vec![(1, "bar".to_string())].into()),]
        );
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct Munch1WithSep<T, P: Parser<Output = T>, Q: Parser> {
    p: P,
    sep: Q,
}
impl<T: Clone, P: Parser<Output = T> + Clone, Q: Parser + Clone> Parser for Munch1WithSep<T, P, Q> {
    type Output = Vec<T>;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        Apply2 {
            px: self.p.clone(),
            qx: Munch {
                px: Skip {
                    skip: self.sep.clone(),
                    p: self.p.clone(),
                },
            },
            f: |x, mut xs| {
                xs.insert(0, x.clone());
                xs
            },
        }
        .parse(tokens)
    }
}
/// Parser that matches a parser zero or more times, separated by another parser, returning a vector of the results of the first parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = munch1_with_sep(literal("foo"), literal(","));
/// assert_eq!(
///     p.parse(
///         vec![
///             (0, "foo".to_string()),
///             (1, ",".to_string()),
///             (2, "foo".to_string()),
///             (3, ",".to_string()),
///             (4, "foo".to_string())
///         ]
///         .into()
///     ),
///     vec![
///         (
///             vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
///             vec![].into()
///         )
///     ]
/// );
/// ```
#[allow(dead_code)]
pub fn munch1_with_sep<T: Clone, P: Parser<Output = T> + Clone, Q: Parser + Clone>(
    p: P,
    sep: Q,
) -> Munch1WithSep<T, P, Q> {
    Munch1WithSep { p, sep }
}
#[cfg(test)]
mod munch1_with_sep {
    #[test]
    fn test() {
        use super::*;
        let p = munch1_with_sep(literal("foo"), literal(","));
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, ",".to_string()),
                    (2, "foo".to_string()),
                    (3, ",".to_string()),
                    (4, "foo".to_string())
                ]
                .into()
            ),
            vec![(
                vec!["foo".to_string(), "foo".to_string(), "foo".to_string()],
                vec![].into()
            )]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, ",".to_string()),
                    (2, "foo".to_string()),
                    (3, ",".to_string()),
                    (4, "bar".to_string())
                ]
                .into()
            ),
            vec![(
                vec!["foo".to_string(), "foo".to_string()],
                vec![(3, ",".to_string()), (4, "bar".to_string())].into()
            )]
        );
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string()),].into()),
            vec![(vec!["foo".to_string()], vec![(1, "bar".to_string())].into())]
        );
        assert_eq!(p.parse(vec![(0, "bar".to_string())].into()), vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct Int32;
impl Parser for Int32 {
    type Output = i32;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        Apply {
            px: Sat {
                pred: |t| t.chars().all(|c| c.is_digit(10)),
            },
            f: |s| s.parse::<i32>().unwrap(),
        }
        .parse(tokens)
    }
}
/// Parser that matches a signed 32-bit integer.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = int32();
/// assert_eq!(
///     p.parse(vec![(0, "123".to_string())].into()),
///     vec![(123, vec![].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn int32() -> Int32 {
    Int32
}
#[cfg(test)]
mod int32 {
    #[test]
    fn test() {
        use super::*;
        let p = int32();
        assert_eq!(
            p.parse(vec![(0, "123".to_string())].into()),
            vec![(123, vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "123foo".to_string())].into()), vec![]);
        assert_eq!(p.parse(vec![(0, "foo".to_string())].into()), vec![]);
    }
}

/// Parser that matches a literal string.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = literal("foo");
/// assert_eq!(
///     p.parse(vec![(0, "   ".to_string())].into()),
///     vec![((), vec![].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn spaces() -> impl Parser<Output = ()> + Clone {
    apply(
        munch1(satisfy(|s| s.chars().all(|c| c.is_whitespace()))),
        |_| (),
    )
}
#[cfg(test)]
mod spaces {
    #[test]
    fn test() {
        use super::*;
        let p = spaces();
        assert_eq!(
            p.parse(vec![(0, " ".to_string())].into()),
            vec![((), vec![].into())]
        );
        assert_eq!(
            p.parse(vec![(0, "  ".to_string())].into()),
            vec![((), vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "foo".to_string())].into()), vec![]);
    }
}
/// Parser that matches a literal string.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = literal("foo");
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string())].into()),
///     vec![("foo".to_string(), vec![].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn string() -> impl Parser<Output = String> + Clone {
    satisfy(|_| true)
}
#[cfg(test)]
mod string {
    #[test]
    fn test() {
        use super::*;
        let p = string();
        assert_eq!(
            p.parse(vec![(0, "foo".to_string())].into()),
            vec![("foo".to_string(), vec![].into())]
        );
        assert_eq!(
            p.parse(vec![(0, "foo bar".to_string())].into()),
            vec![("foo bar".to_string(), vec![].into())]
        );
    }
}
/// Optional parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = optional(literal("foo"));
/// assert_eq!(
///     p.parse(vec![(0, "foo".to_string())].into()),
///     vec![(Some("foo".to_string()), vec![].into())]
/// );
/// assert_eq!(
///     p.parse(vec![(0, "bar".to_string())].into()),
///     vec![(None, vec![(0, "bar".to_string())].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn optional<T: Clone, P>(p: P) -> impl Parser<Output = Option<T>> + Clone
where
    P: Parser<Output = T> + Clone,
{
    altl(apply(p, |x| Some(x)), empty(None))
}
#[cfg(test)]
mod optional {
    #[test]
    fn test() {
        use super::*;
        let p = optional(literal("foo"));
        assert_eq!(
            p.parse(vec![(0, "foo".to_string())].into()),
            vec![(Some("foo".to_string()), vec![].into())]
        );
        assert_eq!(
            p.parse(vec![(0, "bar".to_string())].into()),
            vec![(None, vec![(0, "bar".to_string())].into())]
        );
    }
}

/// Tuple parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = tuple(literal("foo"), literal("bar"));
/// assert_eq!(
///     p.parse(vec![
///         (0, "foo".to_string()),
///         (1, "bar".to_string())
///     ]
///     .into()),
///     vec![(("foo".to_string(), "bar".to_string()), vec![].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn tuple<T: Clone, U: Clone, P, Q>(p: P, q: Q) -> impl Parser<Output = (T, U)> + Clone
where
    P: Parser<Output = T> + Clone,
    Q: Parser<Output = U> + Clone,
{
    bind(p, move |x| apply(q.clone(), move |y| (x.clone(), y)))
}
#[cfg(test)]
mod tuple {
    #[test]
    fn test() {
        use super::*;
        let p = tuple(literal("foo"), literal("bar"));
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "bar".to_string())].into()),
            vec![(("foo".to_string(), "bar".to_string()), vec![].into())]
        );
        assert_eq!(p.parse(vec![(0, "foo".to_string())].into()), vec![]);
        let p = tuple(literal("foo"), int32());
        assert_eq!(
            p.parse(vec![(0, "foo".to_string()), (1, "42".to_string())].into()),
            vec![(("foo".to_string(), 42), vec![].into())]
        );
    }
}

/// Bracketed parser.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = bracket(literal("("), literal("foo"), literal(")"));
/// assert_eq!(
///     p.parse(vec![
///         (0, "(".to_string()),
///         (1, "foo".to_string()),
///         (2, ")".to_string())
///     ]
///     .into()),
///     vec![("foo".to_string(), vec![].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn bracket<T: Clone, L, P, R>(l: L, p: P, r: R) -> impl Parser<Output = T> + Clone
where
    L: Parser + Clone,
    P: Parser<Output = T> + Clone,
    R: Parser + Clone,
{
    skip(l, with(p, r))
}
#[cfg(test)]
mod bracket {
    #[test]
    fn test() {
        use super::*;
        let p = bracket(literal("("), literal("foo"), literal(")"));
        assert_eq!(
            p.parse(
                vec![
                    (0, "(".to_string()),
                    (1, "foo".to_string()),
                    (2, ")".to_string())
                ]
                .into()
            ),
            vec![("foo".to_string(), vec![].into())]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "(".to_string()),
                    (1, "bar".to_string()),
                    (2, ")".to_string())
                ]
                .into()
            ),
            vec![]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "(".to_string()),
                    (2, ")".to_string())
                ]
                .into()
            ),
            vec![]
        );
    }
}

/// Parser that matches a parenthesized expression.
///
/// # Examples
///
/// ```
/// use parser_combinators::*;
/// let p = parens(literal("foo"));
/// assert_eq!(
///     p.parse(vec![
///         (0, "(".to_string()),
///         (1, "foo".to_string()),
///         (2, ")".to_string())
///     ]
///     .into()),
///     vec![("foo".to_string(), vec![].into())]
/// );
/// ```
#[allow(dead_code)]
pub fn parens<T: Clone, P>(p: P) -> impl Parser<Output = T> + Clone
where
    P: Parser<Output = T> + Clone,
{
    bracket(literal("("), p, literal(")"))
}

#[cfg(test)]
mod parens {
    #[test]
    fn test() {
        use super::*;
        let p = parens(literal("foo"));
        assert_eq!(
            p.parse(
                vec![
                    (0, "(".to_string()),
                    (1, "foo".to_string()),
                    (2, ")".to_string())
                ]
                .into()
            ),
            vec![("foo".to_string(), vec![].into())]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "(".to_string()),
                    (1, "bar".to_string()),
                    (2, ")".to_string())
                ]
                .into()
            ),
            vec![]
        );
        assert_eq!(
            p.parse(
                vec![
                    (0, "foo".to_string()),
                    (1, "(".to_string()),
                    (2, ")".to_string())
                ]
                .into()
            ),
            vec![]
        );
    }
}
