mod combinator;

use combinator::*;

fn tokenize(line: &str) -> Vec<String> {
    let mut result = vec![];
    let mut chars = line.chars().peekable();
    let mut token = String::new();

    while let Some(c) = chars.next() {
        match c {
            // 空白読み飛ばし
            ' ' | '\t' => {
                if token.len() > 0 {
                    result.push(token);
                    token = String::new();
                }
            }
            // エスケープ
            '\\' => {
                let c = chars.next().unwrap();
                token.push(c);
            }
            // 文字列
            '"' | '\'' => {
                let quote = c; // クローズ用に取っておく

                if token.len() > 0 {
                    result.push(token);
                    token = String::new();
                }

                token.push(c);

                while let Some(c) = chars.next() {
                    if c == quote {
                        token.push(c);
                        result.push(token);
                        token = String::new();
                        break;
                    }
                    match c {
                        '\\' => {
                            token.push(c);
                            token.push(chars.next().unwrap())
                        }
                        _ => {
                            token.push(c);
                        }
                    }
                }
            }
            '&' | '|' | '(' | ')' | ';' => {
                if token.len() > 0 {
                    result.push(token);
                    token = String::new();
                }
                result.push(c.to_string())
            }
            _ => token.push(c),
        }
    }
    if token.len() > 0 {
        result.push(token);
    }
    result
}
#[cfg(test)]
mod tokenize {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(tokenize("foo"), vec!["foo"]);
        assert_eq!(tokenize(" foo"), vec!["foo"]);
        assert_eq!(tokenize("\tfoo"), vec!["foo"]);
        assert_eq!(tokenize("foo bar buz"), vec!["foo", "bar", "buz"]);
        assert_eq!(
            tokenize("echo \"hello, world\""),
            vec!["echo", "\"hello, world\""]
        );
        assert_eq!(
            tokenize("echo 'hello, world'"),
            vec!["echo", "'hello, world'"]
        );
        assert_eq!(tokenize("echo test&"), vec!["echo", "test", "&"]);
        assert_eq!(
            tokenize("echo \"test\'s\"|grep test"),
            vec!["echo", "\"test's\"", "|", "grep", "test"]
        );
        assert_eq!(
            tokenize("echo test\\'s|grep test"),
            vec!["echo", "test\'s", "|", "grep", "test"]
        );
        assert_eq!(
            tokenize("cd ./home | (make build; make test)"),
            vec!["cd", "./home", "|", "(", "make", "build", ";", "make", "test", ")"]
        );
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum BuiltInCmd {
    Exit(Option<i32>),
    Jobs,
    Fg(i32),
    Cd(String),
}

fn exit_cmd() -> impl Parser<Output = Option<i32>> {
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

fn jobs_cmd() -> impl Parser<Output = String> {
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

fn fg_cmd() -> impl Parser<Output = i32> {
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

fn dir_name() -> impl Parser<Output = String> + Clone {
    satisfy(|s| s.chars().all(|c| !"&|()".contains(c)))
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

fn cd_cmd() -> impl Parser<Output = String> {
    skip(literal("cd"), dir_name())
}
#[cfg(test)]
mod cd_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            cd_cmd().parse(vec![(0, "cd".to_string()), (1, "a".to_string())].into()),
            vec![("a".to_string(), vec![].into())]
        );
        assert_eq!(
            cd_cmd().parse(vec![(0, "cd".to_string()), (1, "./a".to_string())].into()),
            vec![("./a".to_string(), vec![].into())]
        );
        assert_eq!(
            cd_cmd().parse(vec![(0, "cd".to_string()), (1, "&".to_string())].into()),
            vec![]
        );
        assert_eq!(
            cd_cmd().parse(vec![(0, "cd".to_string()), (1, "|".to_string())].into()),
            vec![]
        );
    }
}

fn build_in_cmd() -> impl Parser<Output = BuiltInCmd> {
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
mod build_in_cmd {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            build_in_cmd().parse(vec![(0, "exit".to_string()), (1, "1".to_string())].into()),
            vec![(BuiltInCmd::Exit(Some(1)), vec![].into())]
        );
        assert_eq!(
            build_in_cmd().parse(vec![(0, "exit".to_string()), (1, ";".to_string())].into()),
            vec![(BuiltInCmd::Exit(None), vec![(1, ";".to_string())].into())]
        );
        assert_eq!(
            build_in_cmd().parse(vec![(0, "jobs".to_string())].into()),
            vec![(BuiltInCmd::Jobs, vec![].into())]
        );
        assert_eq!(
            build_in_cmd().parse(vec![(0, "fg".to_string()), (1, "1".to_string())].into()),
            vec![(BuiltInCmd::Fg(1), vec![].into())]
        );
        assert_eq!(
            build_in_cmd().parse(vec![(0, "cd".to_string()), (1, "~/app".to_string())].into()),
            vec![(BuiltInCmd::Cd("~/app".to_string()), vec![].into())]
        );
    }
}
