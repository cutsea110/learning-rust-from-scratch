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
//! - [ ] redirection "<",">",">>"
//! - [x] pipe "|"
//! - [ ] logic operator "&&","||"
//! - [x] background "&"
//! - [ ] semicolon ";"
//!
mod combinator;

use crate::model::*;
use combinator::*;

/// tokenize command line
fn tokenize(line: &str) -> Vec<(usize, String)> {
    use std::mem::take;

    let len = line.len();
    let mut result = vec![];
    let mut chars = line.chars().peekable();
    let mut token = String::new();

    while let Some(c) = chars.next() {
        match c {
            // 空白読み飛ばし
            ' ' | '\t' => {
                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }
            }
            // コマンドライン中のエスケープ(文字列の中ではなく)
            '\\' => {
                token.push('\\');
                let c = chars.next().unwrap();
                token.push(c);
            }
            // 文字列
            '"' | '\'' => {
                let quote = c; // クローズ用に取っておく

                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }

                token.push(quote);

                while let Some(c) = chars.next() {
                    if c == quote {
                        token.push(quote);
                        result.push((len - chars.clone().count() - token.len(), take(&mut token)));
                        break;
                    }
                    match c {
                        '\\' => {
                            token.push('\\');
                            token.push(chars.next().unwrap())
                        }
                        _ => token.push(c),
                    }
                }
            }
            // & もしくは && の場合
            '&' => {
                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }

                if let Some(&c) = chars.peek() {
                    if c == '&' {
                        chars.next();
                        result.push((len - chars.clone().count() - 2, "&&".to_string()));
                        continue;
                    }
                }

                result.push((len - chars.clone().count() - 1, "&".to_string()));
            }
            // これらは 1 文字トークン
            '|' | '(' | ')' | ';' => {
                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }
                result.push((len - chars.clone().count() - 1, c.to_string()))
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
        assert_eq!(tokenize("foo"), vec![(0, "foo".to_string())]);
        assert_eq!(tokenize(" foo"), vec![(1, "foo".to_string())]);
        assert_eq!(tokenize("\tfoo"), vec![(1, "foo".to_string())]);
        assert_eq!(
            tokenize("foo bar buz"),
            vec![
                (0, "foo".to_string()),
                (4, "bar".to_string()),
                (8, "buz".to_string())
            ]
        );
        assert_eq!(
            tokenize("echo \"hello, world\""),
            vec![(0, "echo".to_string()), (5, "\"hello, world\"".to_string())]
        );
        assert_eq!(
            tokenize("echo 'hello, world'"),
            vec![(0, "echo".to_string()), (5, "'hello, world'".to_string())]
        );
        assert_eq!(
            tokenize("echo test&"),
            vec![
                (0, "echo".to_string()),
                (5, "test".to_string()),
                (9, "&".to_string())
            ]
        );
        assert_eq!(
            tokenize("echo \"test\'s\"|grep test"),
            vec![
                (0, "echo".to_string()),
                (5, "\"test's\"".to_string()),
                (13, "|".to_string()),
                (14, "grep".to_string()),
                (19, "test".to_string())
            ]
        );
        assert_eq!(
            tokenize("echo test\\'s|grep test"),
            vec![
                (0, "echo".to_string()),
                (5, "test\\'s".to_string()),
                (12, "|".to_string()),
                (13, "grep".to_string()),
                (18, "test".to_string())
            ]
        );
        assert_eq!(
            tokenize("cd ./home | (make build; make test)"),
            vec![
                (0, "cd".to_string()),
                (3, "./home".to_string()),
                (10, "|".to_string()),
                (12, "(".to_string()),
                (13, "make".to_string()),
                (18, "build".to_string()),
                (23, ";".to_string()),
                (25, "make".to_string()),
                (30, "test".to_string()),
                (34, ")".to_string())
            ]
        );
        assert_eq!(
            tokenize("foo && bar"),
            vec![
                (0, "foo".to_string()),
                (4, "&&".to_string()),
                (7, "bar".to_string())
            ]
        );
        assert_eq!(
            tokenize("foo & & bar"),
            vec![
                (0, "foo".to_string()),
                (4, "&".to_string()),
                (6, "&".to_string()),
                (8, "bar".to_string())
            ]
        );
        assert_eq!(
            tokenize("foo & bar"),
            vec![
                (0, "foo".to_string()),
                (4, "&".to_string()),
                (6, "bar".to_string())
            ]
        );
        assert_eq!(tokenize("foo\0"), vec![(0, "foo\0".to_string())]);
        assert_eq!(tokenize("foo\t"), vec![(0, "foo".to_string())]);
        assert_eq!(tokenize("foo\n"), vec![(0, "foo\n".to_string())]);
    }
}

/// exit command parser
fn exit_cmd() -> impl Parser<Output = Option<i32>> + Clone {
    skip(literal("exit"), optional(int32()))
}
#[cfg(test)]
mod exit_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            exit_cmd().parse(vec![(0, "exit".to_string()), (1, "1".to_string())].into()),
            vec![(Some(1), vec![].into())]
        );
        assert_eq!(
            exit_cmd().parse(vec![(0, "exit".to_string()), (1, "&".to_string())].into()),
            vec![(None, vec![(1, "&".to_string())].into())]
        );
        assert_eq!(
            exit_cmd().parse(vec![(0, "exit".to_string()), (1, "|".to_string())].into()),
            vec![(None, vec![(1, "|".to_string())].into())]
        );
    }
}
/// jobs command parser
fn jobs_cmd() -> impl Parser<Output = String> + Clone {
    literal("jobs")
}
#[cfg(test)]
mod jobs_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            jobs_cmd().parse(vec![(0, "jobs".to_string())].into()),
            vec![("jobs".to_string(), vec![].into())]
        );
        assert_eq!(
            jobs_cmd().parse(vec![(0, "jobs".to_string()), (1, "&".to_string())].into()),
            vec![("jobs".to_string(), vec![(1, "&".to_string())].into())]
        );
        assert_eq!(
            jobs_cmd().parse(vec![(0, "jobs".to_string()), (1, "|".to_string())].into()),
            vec![("jobs".to_string(), vec![(1, "|".to_string())].into())]
        );
    }
}
/// fg command parser
fn fg_cmd() -> impl Parser<Output = i32> + Clone {
    skip(literal("fg"), int32())
}
#[cfg(test)]
mod fg_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            fg_cmd().parse(vec![(0, "fg".to_string()), (1, "1".to_string())].into()),
            vec![(1, vec![].into())]
        );
        assert_eq!(
            fg_cmd().parse(vec![(0, "fg".to_string()), (1, "&".to_string())].into()),
            vec![]
        );
        assert_eq!(
            fg_cmd().parse(vec![(0, "fg".to_string()), (1, "|".to_string())].into()),
            vec![]
        );
    }
}
/// directory name parser
fn dir_name() -> impl Parser<Output = String> + Clone {
    satisfy(|s| !s.chars().any(|c| "&|();".contains(c)))
}
#[cfg(test)]
mod dir_name {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            dir_name().parse(vec![(0, "a".to_string())].into()),
            vec![("a".to_string(), vec![].into())]
        );
        assert_eq!(
            dir_name().parse(vec![(0, "./a".to_string())].into()),
            vec![("./a".to_string(), vec![].into())]
        );
        assert_eq!(dir_name().parse(vec![(0, "&".to_string())].into()), vec![]);
        assert_eq!(dir_name().parse(vec![(0, "|".to_string())].into()), vec![]);
    }
}
/// cd command parser
fn cd_cmd() -> impl Parser<Output = Option<String>> + Clone {
    skip(literal("cd"), optional(dir_name()))
}
#[cfg(test)]
mod cd_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            cd_cmd().parse(vec![(0, "cd".to_string())].into()),
            vec![(None, vec![].into())]
        );
        assert_eq!(
            cd_cmd().parse(vec![(0, "cd".to_string()), (1, "./a".to_string())].into()),
            vec![(Some("./a".to_string()), vec![].into())]
        );
        assert_eq!(
            cd_cmd().parse(vec![(0, "cd".to_string()), (1, "&".to_string())].into()),
            vec![(None, vec![(1, "&".to_string())].into())]
        );
        assert_eq!(
            cd_cmd().parse(vec![(0, "cd".to_string()), (1, "|".to_string())].into()),
            vec![(None, vec![(1, "|".to_string())].into())]
        );
    }
}
/// built-in command parser
fn built_in_cmd() -> impl Parser<Output = BuiltInCmd> + Clone {
    altl(
        apply(exit_cmd(), BuiltInCmd::Exit),
        altl(
            apply(jobs_cmd(), |_| BuiltInCmd::Jobs),
            altl(
                apply(fg_cmd(), BuiltInCmd::Fg),
                apply(cd_cmd(), BuiltInCmd::Cd),
            ),
        ),
    )
}
#[cfg(test)]
mod built_in_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            built_in_cmd().parse(vec![(0, "exit".to_string()), (1, "1".to_string())].into()),
            vec![(BuiltInCmd::Exit(Some(1)), vec![].into())]
        );
        assert_eq!(
            built_in_cmd().parse(vec![(0, "exit".to_string()), (1, ";".to_string())].into()),
            vec![(BuiltInCmd::Exit(None), vec![(1, ";".to_string())].into())]
        );
        assert_eq!(
            built_in_cmd().parse(vec![(0, "jobs".to_string())].into()),
            vec![(BuiltInCmd::Jobs, vec![].into())]
        );
        assert_eq!(
            built_in_cmd().parse(vec![(0, "fg".to_string()), (1, "1".to_string())].into()),
            vec![(BuiltInCmd::Fg(1), vec![].into())]
        );
        assert_eq!(
            built_in_cmd().parse(vec![(0, "cd".to_string()), (1, "~/app".to_string())].into()),
            vec![(BuiltInCmd::Cd(Some("~/app".to_string())), vec![].into())]
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            built_in_cmd().parse(tokenize("exit 1; (ls -laF | grep 'a')& cd ~/app").into()),
            vec![(
                BuiltInCmd::Exit(Some(1)),
                vec![
                    (6, ";".to_string()),
                    (8, "(".to_string()),
                    (9, "ls".to_string()),
                    (12, "-laF".to_string()),
                    (17, "|".to_string()),
                    (19, "grep".to_string()),
                    (24, "'a'".to_string()),
                    (27, ")".to_string()),
                    (28, "&".to_string()),
                    (30, "cd".to_string()),
                    (33, "~/app".to_string())
                ]
                .into()
            )]
        );
    }
}

fn is_separator(s: String) -> bool {
    vec![
        "&".to_string(),
        "|".to_string(),
        "(".to_string(),
        ")".to_string(),
        ";".to_string(),
    ]
    .contains(&s)
}
/// symbol parser
fn symbol() -> impl Parser<Output = String> + Clone {
    satisfy(|s| !is_separator(s))
}
#[cfg(test)]
mod symbol {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            symbol().parse(vec![(0, "ls".to_string())].into()),
            vec![("ls".to_string(), vec![].into())]
        );
        assert_eq!(
            symbol().parse(vec![(0, "ls".to_string()), (1, "-laF".to_string())].into()),
            vec![("ls".to_string(), vec![(1, "-laF".to_string())].into())]
        );
        assert_eq!(symbol().parse(vec![(0, "&".to_string())].into()), vec![]);
        assert_eq!(symbol().parse(vec![(0, "|".to_string())].into()), vec![]);
    }
}

/// external command parser
fn external_cmd() -> impl Parser<Output = ExternalCmd> + Clone {
    apply(munch1(symbol()), |args| ExternalCmd {
        args,
        redirect: None,
    })
}
#[cfg(test)]
mod external_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            external_cmd().parse(vec![(0, "ls".to_string()), (1, "-laF".to_string())].into()),
            vec![(
                ExternalCmd {
                    args: vec!["ls".to_string(), "-laF".to_string()],
                    redirect: None,
                },
                vec![].into()
            )]
        );
        assert_eq!(
            external_cmd().parse(
                vec![
                    (0, "ls".to_string()),
                    (1, "-laF".to_string()),
                    (2, "|".to_string())
                ]
                .into()
            ),
            vec![(
                ExternalCmd {
                    args: vec!["ls".to_string(), "-laF".to_string()],
                    redirect: None,
                },
                vec![(2, "|".to_string())].into()
            )]
        );
    }
}

/// pipe control simbol parser
fn pipe() -> impl Parser<Output = ()> + Clone {
    apply(literal("|"), |_| ())
}
/// job parser
fn job() -> impl Parser<Output = Job> + Clone {
    altl(
        // NOTE: external としてパースされないよう built_in_cmd を先にする
        bind(built_in_cmd(), |cmd| {
            apply(optional(literal("&")), move |bg| Job::BuiltIn {
                cmd: cmd.clone(),
                is_bg: bg.is_some(),
            })
        }),
        bind(munch1_with_sep(external_cmd(), pipe()), |cmds| {
            apply(optional(literal("&")), move |bg| Job::External {
                cmds: cmds.clone(),
                is_bg: bg.is_some(),
            })
        }),
    )
}
#[cfg(test)]
mod job {
    use super::*;

    #[test]
    fn fg_job() {
        assert_eq!(
            job().parse(
                vec![
                    (0, "ls".to_string()),
                    (1, "-laF".to_string()),
                    (2, "|".to_string()),
                    (3, "grep".to_string()),
                    (4, "'a'".to_string())
                ]
                .into()
            ),
            vec![(
                Job::External {
                    cmds: vec![
                        ExternalCmd {
                            args: vec!["ls".to_string(), "-laF".to_string()],
                            redirect: None,
                        },
                        ExternalCmd {
                            args: vec!["grep".to_string(), "'a'".to_string()],
                            redirect: None,
                        }
                    ],
                    is_bg: false,
                },
                vec![].into()
            )]
        );
        assert_eq!(
            job().parse(
                vec![
                    (0, "exit".to_string()),
                    (1, "42".to_string()),
                    (2, "|".to_string()),
                    (3, "grep".to_string()),
                    (4, "'a'".to_string())
                ]
                .into()
            ),
            vec![(
                Job::BuiltIn {
                    cmd: BuiltInCmd::Exit(Some(42)),
                    is_bg: false,
                },
                vec![
                    (2, "|".to_string()),
                    (3, "grep".to_string()),
                    (4, "'a'".to_string())
                ]
                .into()
            )]
        );
    }

    #[test]
    fn bg_job() {
        assert_eq!(
            job().parse(
                vec![
                    (0, "ls".to_string()),
                    (1, "-laF".to_string()),
                    (2, "|".to_string()),
                    (3, "grep".to_string()),
                    (4, "'a'".to_string()),
                    (5, "&".to_string())
                ]
                .into()
            ),
            vec![(
                Job::External {
                    cmds: vec![
                        ExternalCmd {
                            args: vec!["ls".to_string(), "-laF".to_string()],
                            redirect: None,
                        },
                        ExternalCmd {
                            args: vec!["grep".to_string(), "'a'".to_string()],
                            redirect: None,
                        }
                    ],
                    is_bg: true,
                },
                vec![].into()
            )]
        );
        assert_eq!(
            job().parse(
                vec![
                    (0, "exit".to_string()),
                    (1, "42".to_string()),
                    (2, "&".to_string()),
                    (3, "grep".to_string()),
                    (4, "'a'".to_string()),
                    (5, "&".to_string())
                ]
                .into()
            ),
            vec![(
                Job::BuiltIn {
                    cmd: BuiltInCmd::Exit(Some(42)),
                    is_bg: true,
                },
                vec![
                    (3, "grep".to_string()),
                    (4, "'a'".to_string()),
                    (5, "&".to_string())
                ]
                .into()
            )]
        );
    }
}
/// command line parser
fn parse_cmd() -> impl Parser<Output = Vec<Job>> + Clone {
    munch(job())
}
#[cfg(test)]
mod parse_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            parse_cmd().parse(
                vec![
                    (0, "ls".to_string()),
                    (1, "-laF".to_string()),
                    (2, "|".to_string()),
                    (3, "grep".to_string()),
                    (4, "'a'".to_string()),
                    (5, "&".to_string()),
                    (6, "cd".to_string()),
                    (7, "~/app".to_string()),
                    (8, "&".to_string()),
                    (9, "exit".to_string()),
                    (10, "1".to_string())
                ]
                .into()
            ),
            vec![(
                vec![
                    Job::External {
                        cmds: vec![
                            ExternalCmd {
                                args: vec!["ls".to_string(), "-laF".to_string()],
                                redirect: None,
                            },
                            ExternalCmd {
                                args: vec!["grep".to_string(), "'a'".to_string()],
                                redirect: None,
                            }
                        ],
                        is_bg: true,
                    },
                    Job::BuiltIn {
                        cmd: BuiltInCmd::Cd(Some("~/app".to_string())),
                        is_bg: true,
                    },
                    Job::BuiltIn {
                        cmd: BuiltInCmd::Exit(Some(1)),
                        is_bg: false,
                    }
                ],
                vec![].into()
            )]
        );
    }
}

#[derive(Debug, Clone)]
pub enum ParseError {
    Invalid,
    Unknown,
    Unexpected,
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseError::Invalid => write!(f, "invalid"),
            ParseError::Unknown => write!(f, "unknown"),
            ParseError::Unexpected => write!(f, "unexpected"),
        }
    }
}
impl std::error::Error for ParseError {}

/// parsing
pub fn parse(line: &str) -> Result<Vec<Job>, ParseError> {
    let tokens = tokenize(line);
    let mut jobs = parse_cmd().parse(tokens.into());

    match jobs.pop() {
        Some((jobs, rest)) => {
            if rest.is_empty() {
                Ok(jobs)
            } else {
                Err(ParseError::Unknown)
            }
        }
        None => Err(ParseError::Invalid),
    }
}
