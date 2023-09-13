use crate::helper::DynError;
use crate::model;
use crate::parser;
use nix::{
    libc,
    sys::{
        signal::{killpg, signal, SigHandler, Signal},
        wait::{waitpid, WaitPidFlag, WaitStatus},
    },
    unistd::{self, dup2, execvp, fork, pipe, setpgid, tcgetpgrp, tcsetpgrp, ForkResult, Pid},
};
use rustyline::{error::ReadlineError, Editor};
use signal_hook::{consts::*, iterator::Signals};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ffi::CString,
    mem::replace,
    path::PathBuf,
    process::exit,
    sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender},
    thread,
};

/// システムコール呼び出しのラッパ。 EINTR ならリトライ。
fn syscall<F, T>(f: F) -> Result<T, nix::Error>
where
    F: Fn() -> Result<T, nix::Error>,
{
    loop {
        match f() {
            Err(nix::Error::EINTR) => (), // リトライ
            result => return result,
        }
    }
}

/// worker スレッドが受信するメッセージ
enum WorkerMsg {
    Signal(i32),
    Cmd(String),
}

/// main スレッドが受信するメッセージ
enum ShellMsg {
    Continue(i32),
    Quit(i32),
}

pub struct Shell {
    logfile: String, // ログファイル
}

impl Shell {
    pub fn new(logfile: &str) -> Self {
        Self {
            logfile: logfile.to_string(),
        }
    }

    /// main スレッド
    pub fn run(&self) -> Result<(), DynError> {
        // SIGTTOU を無視に設定しないと、 SIGTSTP が配送されてシェルが停止してしまう
        unsafe { signal(Signal::SIGTTOU, SigHandler::SigIgn).unwrap() };

        let mut rl = Editor::<()>::new()?;
        if let Err(e) = rl.load_history(&self.logfile) {
            eprintln!("zerosh: failed to load history: {e}");
        }

        // チャネルを生成して signal_handler と worker スレッドを生成
        let (worker_tx, worker_rx) = channel();
        let (shell_tx, shell_rx) = sync_channel(0);
        spawn_sig_handler(worker_tx.clone())?;
        Worker::new().spawn(worker_rx, shell_tx);

        let exit_val; // 終了コード
        let mut prev = 0; // 直前の終了コード
        loop {
            // 1 行読み込んで、その行を worker スレッドに送信
            let face = if prev == 0 { '\u{1F642}' } else { '\u{1F480}' };
            match rl.readline(&format!("zerosh {face} > ")) {
                Ok(line) => {
                    let line_trimed = line.trim(); // 行頭と行末の空白を削除
                    if line_trimed.is_empty() {
                        continue; // 空行の場合は再読み込み
                    } else {
                        rl.add_history_entry(line_trimed); // ヒストリファイルに追加
                    }

                    // worker スレッドに送信
                    worker_tx.send(WorkerMsg::Cmd(line)).unwrap();
                    match shell_rx.recv().unwrap() {
                        ShellMsg::Continue(n) => prev = n, // 読み込み再開
                        ShellMsg::Quit(n) => {
                            // シェルを終了
                            exit_val = n;
                            break;
                        }
                    }
                }
                // コマンド読み込み時に割り込みが発生した場合は再実行する
                // これは主に Ctrl-C が入力された場合に発生し、誤ってシェルが終了しないようにする
                Err(ReadlineError::Interrupted) => eprintln!("zerosh: press Ctrl-D to exit"),
                // Ctrl-D が入力された場合はシェルを終了する
                Err(ReadlineError::Eof) => {
                    worker_tx.send(WorkerMsg::Cmd("exit".to_string())).unwrap();
                    match shell_rx.recv().unwrap() {
                        ShellMsg::Quit(n) => {
                            // シェルを終了
                            exit_val = n;
                            break;
                        }
                        // exit コマンド実行後は、必ず Quit を受信するはずなので、
                        // それ以外の場合は panic させてプログラムを終了させる
                        _ => panic!("failed to exit"),
                    }
                }
                // なんらかの理由で読み込みに失敗した場合もシェルを終了する
                Err(e) => {
                    eprintln!("zerosh: readline error\n{e}");
                    exit_val = 1;
                    break;
                }
            }
        }

        if let Err(e) = rl.save_history(&self.logfile) {
            eprintln!("zerosh: failed to save history: {e}");
        }
        exit(exit_val);
    }
}

/// signal_handler スレッド
fn spawn_sig_handler(tx: Sender<WorkerMsg>) -> Result<(), DynError> {
    // SIGINT, SIGTSTP は Ctrl-C や Ctrl-Z が入力されてシェルが終了・停止するのを防ぐために受信している
    // SIGCHLD を受信しているのが重要で、子プロセスの状態変化を検知するために必要
    let mut signals = Signals::new(&[SIGINT, SIGTSTP, SIGCHLD])?;
    thread::spawn(move || {
        for sig in signals.forever() {
            // シグナルを受信し worker スレッドに転送する
            tx.send(WorkerMsg::Signal(sig)).unwrap();
        }
    });

    Ok(())
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum ProcState {
    Run,  // 実行中
    Stop, // 停止中
}

#[derive(Debug, Clone)]
struct ProcInfo {
    state: ProcState, // 実行状態
    pgid: Pid,        // プロセスグループ ID
}

#[derive(Debug)]
struct Worker {
    exit_val: i32,   // 終了コード
    fg: Option<Pid>, // フォアグラウンドプロセスのプロセスグループ ID

    // ジョブID から (プロセスグループ ID, 実行コマンド) へのマップ
    jobs: BTreeMap<usize, (Pid, String)>,

    // プロセスグループ ID から (ジョブID, プロセスID) へのマップ
    pgid_to_pids: HashMap<Pid, (usize, HashSet<Pid>)>,

    pid_to_info: HashMap<Pid, ProcInfo>, // プロセスID からプロセスグループID へのマップ
    shell_pgid: Pid,                     // シェルのプロセスグループ ID
}

impl Worker {
    fn new() -> Self {
        Self {
            exit_val: 0,
            fg: None,
            jobs: BTreeMap::new(),
            pgid_to_pids: HashMap::new(),
            pid_to_info: HashMap::new(),

            // libc::STDIN_FILENO に関連付けられた、フォアグラウンドプロセスのプロセスグループID
            // つまりシェルのプロセスグループIDを取得する
            // getpgid でも可能だが、シェルがフォアグラウンドであるかも検査できるので tcgetpgrp を利用している
            // したがって zerosh は制御端末を利用した実行のみをサポートすることになる
            shell_pgid: tcgetpgrp(libc::STDIN_FILENO).unwrap(),
        }
    }

    /// worker スレッドを起動
    fn spawn(mut self, worker_rx: Receiver<WorkerMsg>, shell_tx: SyncSender<ShellMsg>) {
        thread::spawn(move || {
            for msg in worker_rx.iter() {
                match msg {
                    WorkerMsg::Cmd(line) => {
                        match parse_cmd(&line) {
                            Ok(jobs) => {
                                for job in jobs {
                                    match job {
                                        model::Job::BuiltIn { cmd, is_bg } => {
                                            self.built_in_cmd(&cmd, is_bg, &shell_tx);
                                            // 組み込みコマンドなら worker_rx から受信
                                            continue;
                                        }
                                        model::Job::External { cmds, is_bg } => {
                                            if !self.spawn_child(&cmds, is_bg) {
                                                // 子プロセス生成に失敗した場合、シェルからの入力を再開
                                                shell_tx
                                                    .send(ShellMsg::Continue(self.exit_val))
                                                    .unwrap();
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("zerosh: {e}");
                                // コマンドのパースに失敗した場合はシェルからの入力を再開するため
                                // main スレッドに通知する
                                shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
                            }
                        }
                    }
                    WorkerMsg::Signal(SIGCHLD) => {
                        self.wait_child(&shell_tx); // 子プロセスの状態変化を管理
                    }
                    WorkerMsg::Signal(sig) => {
                        // 無視
                        println!("signal: {sig:?} received and ignore it");
                    }
                }
            }
        });
    }

    /// 組み込みコマンドの場合は true を返す
    fn built_in_cmd(
        &mut self,
        cmd: &model::BuiltInCmd,
        is_bg: bool,
        shell_tx: &SyncSender<ShellMsg>,
    ) {
        match cmd {
            model::BuiltInCmd::Exit(n) => self.run_exit(&n, shell_tx),
            model::BuiltInCmd::Jobs => self.run_jobs(shell_tx),
            model::BuiltInCmd::Fg(n) => self.run_fg(&n, shell_tx),
            model::BuiltInCmd::Cd(path) => self.run_cd(path),
        };
    }

    /// 終了コマンドを実行
    fn run_exit(&mut self, n: &Option<i32>, shell_tx: &SyncSender<ShellMsg>) -> bool {
        // 実行中のジョブがある場合は終了しない
        if !self.jobs.is_empty() {
            eprintln!("zerosh: Couldn't quit, there are some running jobs");
            self.exit_val = 1; // 失敗
            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap(); // シェルからの入力を再開
            return true;
        }

        // 終了コードを取得
        let exit_val = n.unwrap_or(self.exit_val);

        shell_tx.send(ShellMsg::Quit(exit_val)).unwrap(); // シェルを終了
        true
    }

    /// ジョブ一覧を表示
    fn run_jobs(&mut self, shell_tx: &SyncSender<ShellMsg>) -> bool {
        for (job_id, (pgid, cmd)) in &self.jobs {
            let state = if self.is_group_stop(*pgid).unwrap() {
                "Stopped"
            } else {
                "Running"
            };
            println!("[{job_id}] {state}\t{cmd}");
        }

        self.exit_val = 0; // 成功
        shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap(); // シェルからの入力を再開
        true
    }

    /// フォアグラウンド実行
    fn run_fg(&mut self, n: &i32, shell_tx: &SyncSender<ShellMsg>) -> bool {
        self.exit_val = 1; // とりあえず失敗に設定
        if let Some((pgid, cmd)) = self.jobs.get(&(*n as usize)) {
            eprintln!("[{n}]: Restart\t{cmd}");

            // フォアグラウンドプロセスに設定
            self.fg = Some(*pgid);
            tcsetpgrp(libc::STDIN_FILENO, *pgid).unwrap();

            // ジョブの実行を再開
            killpg(*pgid, Signal::SIGCONT).unwrap();
            return true;
        }

        // 失敗
        eprintln!("job {n} not found");
        shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap(); // シェルからの入力を再開
        true
    }

    /// ディレクトリ移動
    fn run_cd(&mut self, path: &str) -> bool {
        todo!()
    }

    /// プロセスグループのプロセス全部が停止中なら真
    fn is_group_stop(&self, pgid: Pid) -> Option<bool> {
        for pid in self.pgid_to_pids.get(&pgid)?.1.iter() {
            if self.pid_to_info.get(pid).unwrap().state == ProcState::Run {
                return Some(false);
            }
        }
        Some(true)
    }

    /// 子プロセスを生成。失敗した場合はシェルからの入力を再開する必要がある
    fn spawn_child(&mut self, cmd: &[model::ExternalCmd], is_bg: bool) -> bool {
        todo!()
    }

    /// 子プロセスの状態変化を管理
    fn wait_child(&mut self, shell_tx: &SyncSender<ShellMsg>) {
        todo!()
    }
}

type CmdResult<'a> = Result<Vec<model::Job>, DynError>;

/// コマンドをパース
fn parse_cmd(line: &str) -> CmdResult {
    parser::parse(line).map_err(Into::into)
}
