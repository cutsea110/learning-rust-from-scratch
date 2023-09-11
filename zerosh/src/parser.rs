mod combinator;

use combinator::*;

#[derive(Debug)]
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

fn fg_cmd() -> impl Parser<Output = i32> {
    skip(literal("fd"), int32())
}

fn cd_cmd() -> impl Parser<Output = String> {
    skip(literal("cd"), string())
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
