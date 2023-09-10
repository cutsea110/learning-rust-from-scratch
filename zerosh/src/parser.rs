mod combinator;

use combinator::*;

fn exit_cmd() -> impl Parser<Output = Option<i32>> {
    skip(literal("exit"), optional(int32()))
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
