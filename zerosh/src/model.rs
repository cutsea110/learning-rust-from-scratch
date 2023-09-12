#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BuiltInCmd {
    Exit(Option<i32>),
    Jobs,
    Fg(i32),
    Cd(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExternalCmd {
    pub cmd: String,
    pub opts: Vec<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Cmd {
    BuiltIn(BuiltInCmd),
    External(ExternalCmd),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Job {
    pub cmds: Vec<Cmd>,
    pub is_bg: bool,
}
