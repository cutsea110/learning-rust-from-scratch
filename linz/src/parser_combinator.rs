//! parser combinators
//!
//! ref.) https://bodil.lol/parser-combinators/
//!
type ParseResult<'a, Output> = Result<(&'a str, Output), &'a str>;
trait Parser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output>;

    fn map<F, NewOutput>(self, map_fn: F) -> BoxedParser<'a, NewOutput>
    where
        Self: Sized + 'a,
        Output: 'a,
        NewOutput: 'a,
        F: Fn(Output) -> NewOutput + 'a,
    {
        BoxedParser::new(map(self, map_fn))
    }

    fn pred<F>(self, pred_fn: F) -> BoxedParser<'a, Output>
    where
        Self: Sized + 'a,
        Output: 'a,
        F: Fn(&Output) -> bool + 'a,
    {
        BoxedParser::new(pred(self, pred_fn))
    }

    fn join<Output2, F>(self, parser: F) -> BoxedParser<'a, (Output, Output2)>
    where
        Self: Sized + 'a,
        Output: 'a,
        Output2: 'a,
        F: Parser<'a, Output2> + 'a,
    {
        BoxedParser::new(pair(self, parser))
    }

    fn skip<Output2, F>(self, parser: F) -> BoxedParser<'a, Output2>
    where
        Self: Sized + 'a,
        Output: 'a,
        Output2: 'a,
        F: Parser<'a, Output2> + 'a,
    {
        BoxedParser::new(right(self, parser))
    }

    fn with<Output2, F>(self, parser: F) -> BoxedParser<'a, Output>
    where
        Self: Sized + 'a,
        Output: 'a,
        Output2: 'a,
        F: Parser<'a, Output2> + 'a,
    {
        BoxedParser::new(left(self, parser))
    }

    fn many0(self) -> BoxedParser<'a, Vec<Output>>
    where
        Self: Sized + 'a,
        Output: 'a,
    {
        BoxedParser::new(zero_or_more(self))
    }

    fn many1(self) -> BoxedParser<'a, Vec<Output>>
    where
        Self: Sized + 'a,
        Output: 'a,
    {
        BoxedParser::new(one_or_more(self))
    }

    fn or_else<F>(self, f: F) -> BoxedParser<'a, Output>
    where
        Self: Sized + 'a,
        Output: 'a,
        F: Parser<'a, Output> + 'a,
    {
        BoxedParser::new(altl(self, f))
    }

    fn and_then<F, NextParser, NewOutput>(self, f: F) -> BoxedParser<'a, NewOutput>
    where
        Self: Sized + 'a,
        Output: 'a,
        NewOutput: 'a,
        NextParser: Parser<'a, NewOutput> + 'a,
        F: Fn(Output) -> NextParser + 'a,
    {
        BoxedParser::new(bind(self, f))
    }
}
impl<'a, F, Output> Parser<'a, Output> for F
where
    F: Fn(&'a str) -> ParseResult<Output>, // ParseResult<'a, Output> ??
{
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self(input)
    }
}

struct BoxedParser<'a, Output> {
    parser: Box<dyn Parser<'a, Output> + 'a>,
}
impl<'a, Output> BoxedParser<'a, Output> {
    fn new<P>(parser: P) -> Self
    where
        P: Parser<'a, Output> + 'a,
    {
        BoxedParser {
            parser: Box::new(parser),
        }
    }
}
impl<'a, Output> Parser<'a, Output> for BoxedParser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self.parser.parse(input)
    }
}

fn literal<'a>(expected: &'static str) -> impl Parser<'a, ()> {
    move |input: &'a str| match input.get(0..expected.len()) {
        Some(next) if next == expected => Ok((&input[expected.len()..], ())),
        _ => Err(input),
    }
}
#[cfg(test)]
mod literal {
    use super::*;

    #[test]
    fn test() {
        let parse_joe = literal("Hello Joe!");
        assert_eq!(Ok(("", ())), parse_joe.parse("Hello Joe!"));
        assert_eq!(
            Ok((" Hello Robert!", ())),
            parse_joe.parse("Hello Joe! Hello Robert!")
        );
        assert_eq!(Err("Hello Mike!"), parse_joe.parse("Hello Mike!"));
    }
}

fn identifier(input: &str) -> ParseResult<String> {
    let mut matched = String::new();
    let mut chars = input.chars();

    match chars.next() {
        Some(next) if next.is_alphabetic() => matched.push(next),
        _ => return Err(input),
    }

    while let Some(next) = chars.next() {
        if next.is_alphabetic() || next == '-' {
            matched.push(next);
        } else {
            break;
        }
    }

    let next_index = matched.len();
    Ok((&input[next_index..], matched))
}
#[cfg(test)]
mod identifier {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            Ok(("", "i-am-an-identifier".to_string())),
            identifier("i-am-an-identifier")
        );
        assert_eq!(
            Ok((" entirely an identifier", "not".to_string())),
            identifier("not entirely an identifier")
        );
        assert_eq!(
            Err("!not at all an identifier"),
            identifier("!not at all an identifier")
        );
    }
}

fn pair<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, (R1, R2)>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    move |input| match parser1.parse(input) {
        Ok((next_input, result1)) => match parser2.parse(next_input) {
            Ok((final_input, result2)) => Ok((final_input, (result1, result2))),
            Err(e) => Err(e),
        },
        Err(e) => Err(e),
    }
}
#[cfg(test)]
mod pair {
    use super::*;

    #[test]
    fn test() {
        let tag_opener = pair(literal("<"), identifier);
        assert_eq!(
            Ok(("/>", ((), "my-first-element".to_string()))),
            tag_opener.parse("<my-first-element/>")
        );
        assert_eq!(Err("oops"), tag_opener.parse("oops"));
        assert_eq!(Err("!oops"), tag_opener.parse("<!oops"));
    }
}

fn map<'a, P, F, A, B>(parser: P, map_fn: F) -> impl Parser<'a, B>
where
    P: Parser<'a, A>,
    F: Fn(A) -> B,
{
    move |input| {
        parser
            .parse(input)
            .map(|(next_input, result)| (next_input, map_fn(result)))
    }
}

fn left<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R1>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    map(pair(parser1, parser2), |(left, _right)| left)
}

fn right<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R2>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    map(pair(parser1, parser2), |(_left, right)| right)
}
#[cfg(test)]
mod right {
    use super::*;

    #[test]
    fn test() {
        let tag_opener = right(literal("<"), identifier);
        assert_eq!(
            Ok(("/>", "my-first-element".to_string())),
            tag_opener.parse("<my-first-element/>")
        );
        assert_eq!(Err("oops"), tag_opener.parse("oops"));
        assert_eq!(Err("!oops"), tag_opener.parse("<!oops"));
    }
}

fn one_or_more<'a, P, A>(parser: P) -> impl Parser<'a, Vec<A>>
where
    P: Parser<'a, A>,
{
    move |mut input| {
        let mut result = Vec::new();

        if let Ok((next_input, first_item)) = parser.parse(input) {
            input = next_input;
            result.push(first_item);
        } else {
            return Err(input);
        }

        while let Ok((next_input, next_item)) = parser.parse(input) {
            input = next_input;
            result.push(next_item);
        }

        Ok((input, result))
    }
}
#[cfg(test)]
mod one_or_more {
    use super::*;

    #[test]
    fn test() {
        let parser = one_or_more(literal("ha"));
        assert_eq!(Ok(("", vec![(), (), ()])), parser.parse("hahaha"));
        assert_eq!(Err("ahah"), parser.parse("ahah"));
        assert_eq!(Err(""), parser.parse(""));
    }
}

fn zero_or_more<'a, P, A>(parser: P) -> impl Parser<'a, Vec<A>>
where
    P: Parser<'a, A>,
{
    move |mut input| {
        let mut result = Vec::new();

        while let Ok((next_input, next_item)) = parser.parse(input) {
            input = next_input;
            result.push(next_item);
        }

        Ok((input, result))
    }
}
#[cfg(test)]
mod zero_or_more {
    use super::*;

    #[test]
    fn test() {
        let parser = zero_or_more(literal("ha"));
        assert_eq!(Ok(("", vec![(), (), ()])), parser.parse("hahaha"));
        assert_eq!(Ok(("ahah", vec![])), parser.parse("ahah"));
        assert_eq!(Ok(("", vec![])), parser.parse(""));
    }
}

fn any_char(input: &str) -> ParseResult<char> {
    match input.chars().next() {
        Some(next) => Ok((&input[next.len_utf8()..], next)),
        _ => Err(input),
    }
}
fn pred<'a, P, A, F>(parser: P, predicate: F) -> impl Parser<'a, A>
where
    P: Parser<'a, A>,
    F: Fn(&A) -> bool,
{
    move |input| {
        if let Ok((next_input, value)) = parser.parse(input) {
            if predicate(&value) {
                return Ok((next_input, value));
            }
        }

        Err(input)
    }
}
#[cfg(test)]
mod pred {
    use super::*;

    #[test]
    fn test() {
        let parser = pred(any_char, |c| *c == 'o');
        assert_eq!(Ok(("mg", 'o')), parser.parse("omg"));
        assert_eq!(Err("lol"), parser.parse("lol"));
    }
}

fn whitespace_char<'a>() -> impl Parser<'a, char> {
    any_char.pred(|c| c.is_whitespace())
}

fn space1<'a>() -> impl Parser<'a, Vec<char>> {
    whitespace_char().many1()
}
fn space0<'a>() -> impl Parser<'a, Vec<char>> {
    whitespace_char().many0()
}

fn char<'a>(c: char) -> impl Parser<'a, char> {
    move |input: &'a str| {
        if let Some(next_ch) = input.chars().next() {
            if next_ch == c {
                return Ok((&input[next_ch.len_utf8()..], next_ch));
            }
            return Err(input);
        }

        Err(input)
    }
}
#[cfg(test)]
mod char {
    use super::*;

    #[test]
    fn test() {
        let parser = char('h');
        assert_eq!(Ok(("ello", 'h')), parser.parse("hello"));
        assert_eq!(Err("Hello"), parser.parse("Hello"));
    }
}

fn bracket<'a, R1, R2, R3, P1, P2, P3>(parser1: P1, parser2: P2, parser3: P3) -> impl Parser<'a, R2>
where
    R1: 'a,
    R2: 'a,
    R3: 'a,
    P1: Parser<'a, R1> + 'a,
    P2: Parser<'a, R2> + 'a,
    P3: Parser<'a, R3> + 'a,
{
    parser1.skip(parser2).with(parser3)
}
fn parens<'a, A, P>(parser: P) -> impl Parser<'a, A>
where
    A: 'a,
    P: Parser<'a, A> + 'a,
{
    bracket(char('('), parser, char(')'))
}

fn double_quoted_string<'a>() -> impl Parser<'a, String> {
    char('"')
        .skip(any_char.pred(|c| *c != '"').many0())
        .with(char('"'))
        .map(|chars| chars.into_iter().collect())
}
#[cfg(test)]
mod double_quoted_string {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            Ok(("", "Hello Joe!".to_string())),
            double_quoted_string().parse("\"Hello Joe!\"")
        );
    }
}

fn single_quoted_string<'a>() -> impl Parser<'a, String> {
    char('\'')
        .skip(any_char.pred(|c| *c != '\'').many0())
        .with(char('\''))
        .map(|chars| chars.into_iter().collect())
}
#[cfg(test)]
mod single_quoted_string {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            Ok(("", "Hello Joe!".to_string())),
            single_quoted_string().parse("'Hello Joe!'")
        );
    }
}

fn altl<'a, P1, P2, A>(parser1: P1, parser2: P2) -> impl Parser<'a, A>
where
    P1: Parser<'a, A>,
    P2: Parser<'a, A>,
{
    move |input| match parser1.parse(input) {
        ok @ Ok(_) => ok,
        Err(_) => parser2.parse(input),
    }
}

fn bind<'a, P, F, A, B, NextP>(parser: P, f: F) -> impl Parser<'a, B>
where
    P: Parser<'a, A>,
    NextP: Parser<'a, B>,
    F: Fn(A) -> NextP,
{
    move |input| match parser.parse(input) {
        Ok((next_input, result)) => f(result).parse(next_input),
        Err(e) => Err(e),
    }
}