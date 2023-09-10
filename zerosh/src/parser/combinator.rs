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
pub fn satisfy<F: Fn(String) -> bool>(pred: F) -> Sat<F> {
    Sat { pred }
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
pub fn literal(s: &'static str) -> Lit {
    Lit { s }
}

#[derive(Debug, Clone)]
pub struct Empty<T: Clone>(T);
impl<T: Clone> Parser for Empty<T> {
    type Output = T;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        vec![(self.0.clone(), tokens)]
    }
}
pub fn empty<T: Clone>(x: T) -> Empty<T> {
    Empty(x)
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
pub fn bind<T, P: Parser<Output = T>, F: Fn(T) -> Q, Q: Parser>(px: P, f: F) -> Bind<T, P, F, Q> {
    Bind { px, f }
}

#[derive(Debug, Clone)]
pub struct Apply<T, U, P: Parser<Output = T>> {
    px: P,
    f: fn(T) -> U,
}
impl<T: Clone, U, P: Parser<Output = T>> Parser for Apply<T, U, P> {
    type Output = U;

    fn parse(&self, tokens: VecDeque<Token>) -> Vec<(Self::Output, VecDeque<Token>)> {
        let mut result = vec![];
        for (x, tokens) in self.px.parse(tokens) {
            result.push(((self.f)(x.clone()), tokens));
        }
        result
    }
}
pub fn apply<T, U, P: Parser<Output = T>>(px: P, f: fn(T) -> U) -> Apply<T, U, P> {
    Apply { px, f }
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
pub fn apply2<T, U, V, P: Parser<Output = T>, Q: Parser<Output = U>>(
    px: P,
    qx: Q,
    f: fn(T, U) -> V,
) -> Apply2<T, U, V, P, Q> {
    Apply2 { px, qx, f }
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
pub fn alt<T, P: Parser<Output = T>, Q: Parser<Output = T>>(px: P, qx: Q) -> Alt<T, P, Q> {
    Alt { px, qx }
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
pub fn altl<T, P: Parser<Output = T>, Q: Parser<Output = T>>(px: P, qx: Q) -> AltL<T, P, Q> {
    AltL { px, qx }
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
pub fn ap<T, U, F: Fn(&T) -> U, P: Parser<Output = T>, Q: Parser<Output = F>>(
    px: P,
    pf: Q,
) -> Ap<T, U, F, P, Q> {
    Ap { px, pf }
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
pub fn one_or_more<T: Clone, P: Parser<Output = T> + Clone>(p: P) -> OneOrMore<T, P> {
    OneOrMore { p }
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
pub fn zero_or_more<T: Clone, P: Parser<Output = T> + Clone>(p: P) -> ZeroOrMore<T, P> {
    ZeroOrMore { p }
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
pub fn munch1<T: Clone, P: Parser<Output = T> + Clone>(p: P) -> Munch1<T, P> {
    Munch1 { px: p }
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
pub fn munch<T: Clone, P: Parser<Output = T> + Clone>(p: P) -> Munch<T, P> {
    Munch { px: p }
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
pub fn with<T: Clone, P: Parser<Output = T> + Clone, Q: Parser + Clone>(
    p: P,
    with: Q,
) -> With<T, P, Q> {
    With { p, with }
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
pub fn skip<T: Clone, Q: Parser + Clone, P: Parser<Output = T> + Clone>(
    skip: Q,
    p: P,
) -> Skip<T, Q, P> {
    Skip { skip, p }
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
pub fn one_or_more_with_sep<T: Clone, P: Parser<Output = T> + Clone, Q: Parser + Clone>(
    p: P,
    sep: Q,
) -> OneOrMoreWithSep<T, P, Q> {
    OneOrMoreWithSep { p, sep }
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
pub fn munch1_with_sep<T: Clone, P: Parser<Output = T> + Clone, Q: Parser + Clone>(
    p: P,
    sep: Q,
) -> Munch1WithSep<T, P, Q> {
    Munch1WithSep { p, sep }
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
pub fn int32() -> Int32 {
    Int32
}
