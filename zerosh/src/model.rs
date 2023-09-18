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

    pub fn cmd_line(&self) -> String {
        self.args[0..]
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(" ")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Job {
    BuiltIn { cmd: BuiltInCmd, is_bg: bool },
    External { cmds: Pipeline, is_bg: bool },
}
