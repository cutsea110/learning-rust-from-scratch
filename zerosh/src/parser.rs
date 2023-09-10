mod combinator;

use combinator::*;

fn exit_cmd() -> impl Parser<Output = Option<i32>> {
    skip(literal("exit"), optional(int32()))
}

fn jobs_cmd() -> impl Parser<Output = String> {
    literal("jobs")
}

fn fd_cmd() -> impl Parser<Output = String> {
    literal("fd")
}

fn cd_cmd() -> impl Parser<Output = String> {
    literal("cd")
}
