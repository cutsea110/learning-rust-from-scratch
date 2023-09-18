use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BuiltInCmd {
    Exit(Option<i32>),
    Jobs,
    Fg(i32),
    Cd(Option<String>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Redirection {
    StdOut(String), // > file
    Both(String),   // >& file
    Append(String), // >> file
}

#[derive(Debug, PartialEq, Clone)]
pub enum Pipe {
    StdOut, // |
    Both,   // |&
}

#[derive(Debug, PartialEq, Clone)]
pub enum Pipeline {
    Src(ExternalCmd),
    Out(Box<Pipeline>, ExternalCmd),
    Both(Box<Pipeline>, ExternalCmd),
}
impl Pipeline {
    // TODO: Display にする
    pub fn cmd_line(&self) -> String {
        match &self {
            Pipeline::Src(cmd) => format!("{}", cmd),
            Pipeline::Out(p, cmd) => format!("{} | {}", p.cmd_line(), cmd),
            Pipeline::Both(p, cmd) => format!("{} |& {}", p.cmd_line(), cmd),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExternalCmd {
    pub args: Vec<String>,
    pub redirect: Option<Redirection>,
}
impl ExternalCmd {
    pub fn filename(&self) -> &str {
        assert_ne!(self.args.len(), 0);
        &self.args[0]
    }
}
impl fmt::Display for ExternalCmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.args[0..]
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(" ")
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Job {
    BuiltIn { cmd: BuiltInCmd, is_bg: bool },
    External { cmds: Pipeline, is_bg: bool },
}
