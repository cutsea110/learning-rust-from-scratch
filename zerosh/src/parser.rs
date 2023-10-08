//! Parser for shell command line.
//!
//! # BNF
//!
//! # Built-in command
//!
//! - [x] exit
//! - [x] jobs
//! - [x] fg
//! - [x] cd
//!
//! # Priority of control code
//!
//! - [ ] parenthesis "()","{}","``","$()"
//! - [x] redirection ">",">>",">&"
//! - [x] pipe "|","|&"
//! - [ ] logic operator "&&","||"
//! - [x] background "&"
//! - [ ] semicolon ";"
//!
use crate::model::*;
use parser_combinator::*;

/// exit command parser
fn exit_cmd<'a>() -> impl Parser<'a, Option<i32>> {
    |input| {
        let (next_i, _) = space0().parse(input)?;
        let (next_i, _) = keyword("exit").parse(next_i)?;

        opt(space1().skip(int32)).parse(next_i)
    }
}
#[cfg(test)]
mod exit_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(exit_cmd().parse("exit 1"), Ok(("", Some(1))));
        assert_eq!(exit_cmd().parse("exit &"), Ok((" &", None)));
        assert_eq!(exit_cmd().parse("exit |"), Ok((" |", None)));
    }
}
/// jobs command parser
fn jobs_cmd<'a>() -> impl Parser<'a, &'a str> {
    |input| {
        let (next_i, _) = space0().parse(input)?;

        keyword("jobs").parse(next_i)
    }
}
#[cfg(test)]
mod jobs_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(jobs_cmd().parse("jobs"), Ok(("", "jobs")));
        assert_eq!(jobs_cmd().parse("jobs &"), Ok((" &", "jobs")));
        assert_eq!(jobs_cmd().parse("jobs |"), Ok((" |", "jobs")));
    }
}
/// fg command parser
fn fg_cmd<'a>() -> impl Parser<'a, i32> {
    |input| {
        let (next_i, _) = space0().parse(input)?;
        let (next_i, _) = keyword("fg").parse(next_i)?;
        let (next_i, _) = space1().parse(next_i)?;

        int32(next_i)
    }
}
#[cfg(test)]
mod fg_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(fg_cmd().parse("fg 1"), Ok(("", 1)));
        assert_eq!(fg_cmd().parse("fg &"), Err("&"));
        assert_eq!(fg_cmd().parse("fg |"), Err("|"));
    }
}
/// path name parser
fn path_name<'a>() -> impl Parser<'a, String> {
    |input| {
        let (next_i, _) = space0().parse(input)?;

        any_char
            .pred(|c| !"&|()<>;".contains(*c) && !c.is_whitespace())
            .many1()
            .map(|s| s.into_iter().collect::<String>())
            .parse(next_i)
    }
}
#[cfg(test)]
mod path_name {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(path_name().parse("a"), Ok(("", "a".to_string())));
        assert_eq!(path_name().parse("./a"), Ok(("", "./a".to_string())));
        assert_eq!(path_name().parse("&"), Err("&"));
        assert_eq!(path_name().parse("|"), Err("|"));
    }
}
/// cd command parser
fn cd_cmd<'a>() -> impl Parser<'a, Option<String>> {
    |input| {
        let (next_i, _) = space0().parse(input)?;
        let (next_i, _) = keyword("cd").parse(next_i)?;

        opt(path_name()).parse(next_i)
    }
}
#[cfg(test)]
mod cd_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(cd_cmd().parse("cd"), Ok(("", None)));
        assert_eq!(cd_cmd().parse("cd ./a"), Ok(("", Some("./a".to_string()))));
        assert_eq!(cd_cmd().parse("cd &"), Ok((" &", None)));
        assert_eq!(cd_cmd().parse("cd |"), Ok((" |", None)));
    }
}
/// built-in command parser
fn built_in_cmd<'a>() -> impl Parser<'a, BuiltInCmd> {
    exit_cmd()
        .map(BuiltInCmd::Exit)
        .or_else(jobs_cmd().map(|_| BuiltInCmd::Jobs))
        .or_else(fg_cmd().map(BuiltInCmd::Fg))
        .or_else(cd_cmd().map(BuiltInCmd::Cd))
}
#[cfg(test)]
mod built_in_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            built_in_cmd().parse("exit 1"),
            Ok(("", BuiltInCmd::Exit(Some(1))))
        );
        assert_eq!(
            built_in_cmd().parse("exit ;"),
            Ok((" ;", BuiltInCmd::Exit(None)))
        );
        assert_eq!(built_in_cmd().parse("jobs"), Ok(("", BuiltInCmd::Jobs)));
        assert_eq!(built_in_cmd().parse("fg 1"), Ok(("", BuiltInCmd::Fg(1))));
        assert_eq!(
            built_in_cmd().parse("cd ~/app"),
            Ok(("", BuiltInCmd::Cd(Some("~/app".to_string()))))
        );
        assert_eq!(
            built_in_cmd().parse("exit 1; (ls -laF | grep 'a')& cd ~/app"),
            Ok((
                "; (ls -laF | grep 'a')& cd ~/app",
                BuiltInCmd::Exit(Some(1))
            ))
        );
    }
}

/// symbol parser
fn symbol<'a>() -> impl Parser<'a, String> {
    |input| {
        let (next_i, _) = space0().parse(input)?;

        any_char
            .pred(|c| !"&|()<>;".contains(*c) && !c.is_whitespace())
            .many1()
            .map(|cs| cs.into_iter().collect::<String>())
            .parse(next_i)
    }
}
#[cfg(test)]
mod symbol {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(symbol().parse("ls"), Ok(("", "ls".to_string())));
        assert_eq!(symbol().parse("ls -laF"), Ok((" -laF", "ls".to_string())));
        assert_eq!(symbol().parse("&"), Err("&"));
        assert_eq!(symbol().parse("|"), Err("|"));
    }
}

fn redirect<'a>() -> impl Parser<'a, Redirection> {
    |input| {
        let (next_i, _) = space0().parse(input)?;
        let (next_i, tok) = keyword(">&")
            .or_else(keyword(">>"))
            .or_else(keyword(">")) // 短いのを最後にしないと全部 '>' にマッチしてしまう
            .parse(next_i)?;
        let (next_i, _) = space0().parse(next_i)?;
        let (next_i, file) = path_name().parse(next_i)?;

        let red = match tok {
            ">" => Redirection::StdOut(file),
            ">&" => Redirection::Both(file),
            ">>" => Redirection::Append(file),
            _ => unreachable!(),
        };

        Ok((next_i, red))
    }
}
#[cfg(test)]
mod redirect {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            redirect().parse("> a.txt"),
            Ok(("", Redirection::StdOut("a.txt".to_string())))
        );
        assert_eq!(
            redirect().parse(">& a.txt"),
            Ok(("", Redirection::Both("a.txt".to_string())))
        );
    }
}

/// external command parser
fn external_cmd<'a>() -> impl Parser<'a, ExternalCmd> {
    symbol().many1().and_then(|args| {
        opt(redirect()).map(move |out| ExternalCmd {
            args: args.clone(),
            redirect: out,
        })
    })
}
#[cfg(test)]
mod external_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            external_cmd().parse("ls -laF"),
            Ok((
                "",
                ExternalCmd {
                    args: vec!["ls".to_string(), "-laF".to_string()],
                    redirect: None,
                }
            ))
        );
        assert_eq!(
            external_cmd().parse("ls -laF |"),
            Ok((
                " |",
                ExternalCmd {
                    args: vec!["ls".to_string(), "-laF".to_string()],
                    redirect: None,
                }
            ))
        );
        assert_eq!(
            external_cmd().parse("ls -laF > a.log"),
            Ok((
                "",
                ExternalCmd {
                    args: vec!["ls".to_string(), "-laF".to_string()],
                    redirect: Some(Redirection::StdOut("a.log".to_string())),
                }
            ))
        );
    }
}

/// pipe control simbol parser
fn pipe<'a>() -> impl Parser<'a, Pipe> {
    |input| {
        let (next_i, _) = space0().parse(input)?;
        // '|' は最後にしないとなんでも '|' にマッチしてしまう
        let (next_i, p) = keyword("|&").or_else(keyword("|")).parse(next_i)?;

        match p {
            "|" => Ok((next_i, Pipe::StdOut)),
            "|&" => Ok((next_i, Pipe::Both)),
            _ => unreachable!(),
        }
    }
}
#[cfg(test)]
mod pipe {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(pipe().parse("|"), Ok(("", Pipe::StdOut)));
        assert_eq!(pipe().parse("|&"), Ok(("", Pipe::Both)));
    }
}

/// pipeline parser
fn pipeline<'a>() -> impl Parser<'a, Pipeline> {
    |input| {
        let (next_i, cmd) = external_cmd().parse(input)?;
        let (next_i, cmds) = pipe().join(external_cmd()).many0().parse(next_i)?;

        let mut acc = Pipeline::Src(cmd.clone());
        for (p, cmd) in cmds {
            acc = match &p {
                Pipe::StdOut => Pipeline::Out(Box::new(acc), cmd),
                Pipe::Both => Pipeline::Both(Box::new(acc), cmd),
            };
        }
        Ok((next_i, acc))
    }
}
#[cfg(test)]
mod pipeline {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            pipeline().parse("foo | bar"),
            Ok((
                "",
                Pipeline::Out(
                    Box::new(Pipeline::Src(ExternalCmd {
                        args: vec!["foo".to_string()],
                        redirect: None,
                    })),
                    ExternalCmd {
                        args: vec!["bar".to_string()],
                        redirect: None,
                    }
                )
            ))
        );
        assert_eq!(
            pipeline().parse("foo |& bar"),
            Ok((
                "",
                Pipeline::Both(
                    Box::new(Pipeline::Src(ExternalCmd {
                        args: vec!["foo".to_string()],
                        redirect: None,
                    })),
                    ExternalCmd {
                        args: vec!["bar".to_string()],
                        redirect: None,
                    }
                )
            ))
        );
        assert_eq!(
            pipeline().parse("foo | bar |& buz"),
            Ok((
                "",
                Pipeline::Both(
                    Box::new(Pipeline::Out(
                        Box::new(Pipeline::Src(ExternalCmd {
                            args: vec!["foo".to_string()],
                            redirect: None,
                        })),
                        ExternalCmd {
                            args: vec!["bar".to_string()],
                            redirect: None,
                        }
                    )),
                    ExternalCmd {
                        args: vec!["buz".to_string()],
                        redirect: None,
                    }
                )
            ))
        );
    }
}

/// job parser
fn job<'a>() -> impl Parser<'a, Job> {
    built_in_cmd()
        .and_then(|cmd| {
            lexeme(opt(literal("&"))).map(move |bg| Job::BuiltIn {
                cmd: cmd.clone(),
                is_bg: bg.is_some(),
            })
        })
        .or_else(pipeline().and_then(|cmds| {
            lexeme(opt(literal("&"))).map(move |bg| Job::External {
                cmds: cmds.clone(),
                is_bg: bg.is_some(),
            })
        }))
}
#[cfg(test)]
mod job {
    use super::*;

    #[test]
    fn fg_job() {
        assert_eq!(
            job().parse("ls -laF | grep a"),
            Ok((
                "",
                Job::External {
                    cmds: Pipeline::Out(
                        Box::new(Pipeline::Src(ExternalCmd {
                            args: vec!["ls".to_string(), "-laF".to_string()],
                            redirect: None,
                        })),
                        ExternalCmd {
                            args: vec!["grep".to_string(), "a".to_string()],
                            redirect: None,
                        }
                    ),
                    is_bg: false,
                }
            ))
        );
        assert_eq!(
            job().parse("exit 42 | grep a"),
            Ok((
                "| grep a",
                Job::BuiltIn {
                    cmd: BuiltInCmd::Exit(Some(42)),
                    is_bg: false,
                }
            ))
        );
        assert_eq!(
            job().parse("exit"),
            Ok((
                "",
                Job::BuiltIn {
                    cmd: BuiltInCmd::Exit(None),
                    is_bg: false,
                }
            ))
        );
        assert_eq!(
            job().parse("jobs"),
            Ok((
                "",
                Job::BuiltIn {
                    cmd: BuiltInCmd::Jobs,
                    is_bg: false,
                }
            ))
        );
        assert_eq!(
            job().parse("fg 1"),
            Ok((
                "",
                Job::BuiltIn {
                    cmd: BuiltInCmd::Fg(1),
                    is_bg: false,
                }
            ))
        );
        assert_eq!(
            job().parse("cd"),
            Ok((
                "",
                Job::BuiltIn {
                    cmd: BuiltInCmd::Cd(None),
                    is_bg: false,
                }
            ))
        );
        assert_eq!(
            job().parse("cd ./app"),
            Ok((
                "",
                Job::BuiltIn {
                    cmd: BuiltInCmd::Cd(Some("./app".to_string())),
                    is_bg: false,
                }
            ))
        );
    }

    #[test]
    fn bg_job() {
        assert_eq!(
            job().parse("ls -laF | grep a &"),
            Ok((
                "",
                Job::External {
                    cmds: Pipeline::Out(
                        Box::new(Pipeline::Src(ExternalCmd {
                            args: vec!["ls".to_string(), "-laF".to_string()],
                            redirect: None,
                        })),
                        ExternalCmd {
                            args: vec!["grep".to_string(), "a".to_string()],
                            redirect: None,
                        }
                    ),
                    is_bg: true,
                }
            ))
        );
        assert_eq!(
            job().parse("exit 42 & grep a &"),
            Ok((
                " grep a &",
                Job::BuiltIn {
                    cmd: BuiltInCmd::Exit(Some(42)),
                    is_bg: true,
                }
            ))
        );
    }
}
/// command line parser
fn parse_cmd<'a>() -> impl Parser<'a, Vec<Job>> {
    job().many0()
}
#[cfg(test)]
mod parse_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            parse_cmd().parse("ls -laF | grep a & cd ~/app & exit 1"),
            Ok((
                "",
                vec![
                    Job::External {
                        cmds: Pipeline::Out(
                            Box::new(Pipeline::Src(ExternalCmd {
                                args: vec!["ls".to_string(), "-laF".to_string()],
                                redirect: None,
                            })),
                            ExternalCmd {
                                args: vec!["grep".to_string(), "a".to_string()],
                                redirect: None,
                            }
                        ),
                        is_bg: true,
                    },
                    Job::BuiltIn {
                        cmd: BuiltInCmd::Cd(Some("~/app".to_string())),
                        is_bg: true
                    },
                    Job::BuiltIn {
                        cmd: BuiltInCmd::Exit(Some(1)),
                        is_bg: false
                    },
                ]
            ))
        );
    }
}

/// parsing
pub fn parse<'a>(input: &'a str) -> ParseResult<'a, Vec<Job>> {
    parse_cmd().parse(input)
}
