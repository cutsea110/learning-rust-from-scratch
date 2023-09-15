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
impl ExternalCmd {
    pub fn filename(&self) -> &str {
        assert_ne!(self.args.len(), 0);
        &self.args[0]
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Job {
    BuiltIn { cmd: BuiltInCmd, is_bg: bool },
    External { cmds: Vec<ExternalCmd>, is_bg: bool },
}
