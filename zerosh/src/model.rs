#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BuiltInCmd {
    Exit(Option<i32>),
    Jobs,
    Fg(i32),
    Cd(Option<String>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExternalCmd {
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Job {
    BuiltIn { cmd: BuiltInCmd, is_bg: bool },
    External { cmds: Vec<ExternalCmd>, is_bg: bool },
}
