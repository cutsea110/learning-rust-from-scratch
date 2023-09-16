use crate::helper::DynError;
use crate::model;
use crate::parser;
use nix::{
    libc,
    sys::{
        signal::{killpg, signal, SigHandler, Signal},
        wait::{waitpid, WaitPidFlag, WaitStatus},
    },
    unistd::{
        close, dup2, execvp, fork, getpgid, getpid, pipe, setpgid, tcgetpgrp, tcsetpgrp,
        ForkResult, Pid,
    },
};
use rustyline::{error::ReadlineError, Editor};
use signal_hook::{consts::*, iterator::Signals};
use std::collections::VecDeque;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ffi::CString,
    mem::replace,
    path::PathBuf,
    process::exit,
    sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender},
    thread,
};

const NAME: &str = "zerosh";

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
            eprintln!("{NAME}: failed to load history: {e}");
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
            match rl.readline(&format!("{NAME} {face} > ")) {
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
                Err(ReadlineError::Interrupted) => eprintln!("{NAME}: press Ctrl-D to exit"),
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
                    eprintln!("{NAME}: readline error\n{e}");
                    exit_val = 1;
                    break;
                }
            }
        }

        if let Err(e) = rl.save_history(&self.logfile) {
            eprintln!("{NAME}: failed to save history: {e}");
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
                                eprintln!("{NAME}: {e}");
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
            model::BuiltInCmd::Cd(path) => self.run_cd(path, shell_tx),
        };
    }

    /// 終了コマンドを実行
    fn run_exit(&mut self, n: &Option<i32>, shell_tx: &SyncSender<ShellMsg>) -> bool {
        // 実行中のジョブがある場合は終了しない
        if !self.jobs.is_empty() {
            eprintln!("{NAME}: Couldn't quit, there are some running jobs");
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
    fn run_cd(&mut self, path: &Option<String>, shell_tx: &SyncSender<ShellMsg>) -> bool {
        let path = match path {
            // 引数が指定されていない場合、ホームディレクトリか / に移動
            None => dirs::home_dir()
                .or_else(|| Some(PathBuf::from("/")))
                .unwrap(),
            Some(path) => PathBuf::from(path),
        };

        // カレントディレクトリを変更
        if let Err(e) = std::env::set_current_dir(&path) {
            self.exit_val = 1; // 失敗
            eprintln!("failed to change directory to {path:?}: {e}");
        } else {
            self.exit_val = 0; // 成功
        }

        shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap(); // シェルからの入力を再開
        true
    }

    /// 子プロセスを生成。失敗した場合はシェルからの入力を再開する必要がある
    fn spawn_child(&mut self, cmd: &[model::ExternalCmd], is_bg: bool) -> bool {
        assert_ne!(cmd.len(), 0);

        // ジョブ ID を取得
        let job_id = if let Some(id) = self.get_new_job_id() {
            id
        } else {
            eprintln!("{NAME}: Couldn't spawn child process, too many jobs already exists");
            return false;
        };

        let pgid;
        let mut pids = HashMap::new();
        // ジョブを処理するベースとなるプロセスを生成
        match fork_exec(Pid::from_raw(0), &cmd, &mut pids) {
            Ok(child) => {
                pgid = child;
            }
            Err(e) => {
                eprintln!("{NAME}: Failed to fork: {e}");
                return false;
            }
        }

        if !is_bg {
            // ジョブ情報を追加して子プロセスをフォアグラウンドプロセスグループにする
            self.fg = Some(pgid);
            let line = cmd
                .iter()
                .map(|x| x.cmd_line())
                .collect::<Vec<String>>()
                .join(" | ");
            self.insert_job(job_id, pgid, pids, &line);
            tcsetpgrp(libc::STDIN_FILENO, pgid).unwrap();
        }

        true
    }

    /// ジョブの管理
    /// 引数には変化のあったジョブとプロセスグループを指定
    ///
    /// - フォアグラウンドプロセスが空の場合、シェルをフォアグラウンドに設定
    /// - フォアグラウンドプロセスがすべて停止中の場合、シェルをフォアグラウンドに設定
    fn manage_job(&mut self, job_id: usize, pgid: Pid, shell_tx: &SyncSender<ShellMsg>) {
        let is_fg = self.fg.map_or(false, |x| pgid == x); // フォアグラウンドのプロセスか?
        let line = &self.jobs.get(&job_id).unwrap().1;
        if is_fg {
            // 状態が変化したプロセスはフォアグラウンドに設定
            if self.is_group_empty(pgid) {
                // フォアグラウンドプロセスが空の場合、
                // ジョブ情報を削除してシェルをフォアグラウンドに設定
                eprintln!("\n[{job_id}] Done\t{line}");
                self.remove_job(job_id);
                self.set_shell_fg(shell_tx);
            } else if self.is_group_stop(pgid).unwrap() {
                // フォアグラウンドプロセスがすべて停止中の場合、シェルをフォアグラウンドに設定
                eprintln!("\n[{job_id}Stopped\t{line}");
                self.set_shell_fg(shell_tx);
            }
        } else {
            // プロセスグループが空の場合、ジョブ情報を削除
            if self.is_group_empty(pgid) {
                eprintln!("\n[{job_id}] Done\t{line}");
                self.remove_job(job_id);
            }
        }
    }

    /// 新たなジョブ情報を追加
    fn insert_job(&mut self, job_id: usize, pgid: Pid, pids: HashMap<Pid, ProcInfo>, line: &str) {
        assert!(!self.jobs.contains_key(&job_id));
        self.jobs.insert(job_id, (pgid, line.to_string())); // ジョブ情報を追加

        let mut procs = HashSet::new(); // pgid_to_pids へ追加するプロセス
        for (pid, info) in pids {
            procs.insert(pid);

            assert!(!self.pid_to_info.contains_key(&pid));
            self.pid_to_info.insert(pid, info); // プロセス情報を追加
        }

        assert!(!self.pgid_to_pids.contains_key(&pgid));
        self.pgid_to_pids.insert(pgid, (job_id, procs)); // プロセスグループ情報を追加
    }

    /// プロセスの実行状態を設定し、依然の状態を返す
    /// pid が存在しない場合は None を返す
    fn set_pid_state(&mut self, pid: Pid, state: ProcState) -> Option<ProcState> {
        let info = self.pid_to_info.get_mut(&pid)?;
        Some(replace(&mut info.state, state))
    }

    /// プロセスの情報を削除し、削除できた場合はプロセスの所属する
    /// (ジョブ ID, プロセスグループ ID) を返す
    /// 存在しない場合は None を返す
    fn remove_pid(&mut self, pid: Pid) -> Option<(usize, Pid)> {
        let pgid = self.pid_to_info.get(&pid)?.pgid;
        let it = self.pgid_to_pids.get_mut(&pgid)?;
        it.1.remove(&pid); // プロセスグループから pid を削除
        let job_id = it.0; // ジョブ ID を取得
        Some((job_id, pgid))
    }

    /// ジョブ情報を削除し、関連するプロセスグループの情報も削除
    fn remove_job(&mut self, job_id: usize) {
        if let Some((pgid, _)) = self.jobs.remove(&job_id) {
            if let Some((_, pids)) = self.pgid_to_pids.remove(&pgid) {
                assert!(pids.is_empty()); // ジョブを削除するときはプロセスグループも空のはず
            }
        }
    }

    /// 空のプロセスグループなら真
    fn is_group_empty(&self, pgid: Pid) -> bool {
        self.pgid_to_pids.get(&pgid).unwrap().1.is_empty()
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

    /// シェルをフォアグラウンドに設定
    fn set_shell_fg(&mut self, shell_tx: &SyncSender<ShellMsg>) {
        self.fg = None;
        tcsetpgrp(libc::STDIN_FILENO, self.shell_pgid).unwrap();
        shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
    }

    fn get_new_job_id(&self) -> Option<usize> {
        for i in 0..=usize::MAX {
            if !self.jobs.contains_key(&i) {
                return Some(i);
            }
        }
        None
    }

    /// 子プロセスの状態変化を管理
    fn wait_child(&mut self, shell_tx: &SyncSender<ShellMsg>) {
        // WUNTRACED: 子プロセスの停止
        // WNOHANG: ブロックしない
        // WCONTINUED: 実行再開
        let flag = Some(WaitPidFlag::WUNTRACED | WaitPidFlag::WNOHANG | WaitPidFlag::WCONTINUED);

        loop {
            // pid = -1 指定によりすべての子プロセスの状態変化を待つ
            // waitpid は終了したプロセスのリソース開放も行う
            // これを忘れるとゾンビプロセスになり無駄にリソースを消費する
            // WNOHANG を指定しているので、子プロセスの状態に変化がない場合は即座に返る
            // これにより worker はシグナルとコマンドライン実行の両方を並行に処理できる
            match syscall(|| waitpid(Pid::from_raw(-1), flag)) {
                Ok(WaitStatus::Exited(pid, status)) => {
                    // プロセスが終了
                    self.exit_val = status; // 終了コードを保存
                    self.process_term(pid, shell_tx);
                }
                Ok(WaitStatus::Signaled(pid, sig, core)) => {
                    // プロセスがシグナルにより終了
                    eprintln!(
                        "\n{NAME}: Child process terminated by signal{}: pid = {pid}, signal = {sig}",
			if core { " (core dumped)" } else { "" },
                    );
                    self.exit_val = sig as i32 + 128; // 終了コードを保存
                    self.process_term(pid, shell_tx);
                }
                // プロセスが停止
                Ok(WaitStatus::Stopped(pid, sig)) => self.process_stop(pid, shell_tx),
                Ok(WaitStatus::Continued(pid)) => self.process_continue(pid),
                Ok(WaitStatus::StillAlive) => return, // wait すべき子プロセスはいない
                Err(nix::Error::ECHILD) => return,    // 子プロセスはいない
                Err(e) => {
                    eprintln!("\n{NAME}: Failed to wait: {e}");
                    exit(1);
                }
                #[cfg(any(target_os = "linux", target_os = "android"))]
                Ok(WaitStatus::PtraceEvent(pid, _, _) | WaitStatus::PtraceSyscall(pid)) => {
                    self.process_stop(pid, shell_tx)
                }
            }
        }
    }

    // プロセスの終了処理
    fn process_term(&mut self, pid: Pid, shell_tx: &SyncSender<ShellMsg>) {
        // プロセス ID を削除し、必要ならフォアグラウンドプロセスをシェルに設定
        if let Some((job_id, pgid)) = self.remove_pid(pid) {
            self.manage_job(job_id, pgid, shell_tx);
        }
    }

    // プロセスの停止処理
    fn process_stop(&mut self, pid: Pid, shell_tx: &SyncSender<ShellMsg>) {
        self.set_pid_state(pid, ProcState::Stop); // プロセスを停止中に設定
        let pgid = self.pid_to_info.get(&pid).unwrap().pgid; // プロセスグループ ID を取得
        let job_id = self.pgid_to_pids.get(&pgid).unwrap().0; // ジョブ ID を取得
        self.manage_job(job_id, pgid, shell_tx); // 必要ならフォアグラウンドプロセスをシェルに設定
    }

    // プロセスの再開処理
    fn process_continue(&mut self, pid: Pid) {
        self.set_pid_state(pid, ProcState::Run); // プロセスを実行中に設定
    }
}

fn do_pipeline(cmds: &mut VecDeque<model::ExternalCmd>, pids: &mut HashMap<Pid, ProcInfo>) {
    let cmd = cmds.pop_back().unwrap();
    let filename = CString::new(cmd.filename()).unwrap();
    let args = cmd
        .args
        .iter()
        .map(|s| CString::new(s.as_str()).unwrap())
        .collect::<Vec<_>>();

    // TODO: Stdout 以外のリダイレクトにも対応する
    let handle_redirect = || {
        if let Some(model::Redirection::Stdout(ref out)) = cmd.redirect {
            let fd = syscall(move || {
                nix::fcntl::open(
                    out.as_str(),
                    nix::fcntl::OFlag::O_WRONLY | nix::fcntl::OFlag::O_CREAT,
                    nix::sys::stat::Mode::S_IRWXU,
                )
            })
            .unwrap();
            syscall(|| {
                close(libc::STDOUT_FILENO).unwrap();
                dup2(fd, libc::STDOUT_FILENO).unwrap();
                close(fd)
            })
            .unwrap();
        }
    };

    if cmds.is_empty() {
        // リダイレクト処理
        handle_redirect();

        match execvp(&filename, &args) {
            Err(e) => {
                eprintln!("{NAME}: Failed to exec: {e}");
                exit(1);
            }
            Ok(_) => unreachable!(),
        }
    } else {
        let p = pipe().unwrap();
        match syscall(|| unsafe { fork() }).unwrap() {
            ForkResult::Child => {
                // 子プロセスならパイプを stdout に dup2 して再帰
                syscall(|| {
                    close(p.0).unwrap();
                    dup2(p.1, libc::STDOUT_FILENO).unwrap();
                    close(p.1)
                })
                .unwrap();

                do_pipeline(cmds, pids);
            }
            ForkResult::Parent { child } => {
                // リダイレクト処理
                handle_redirect();

                // 親プロセスならパイプを stdin に dup2 して最後のコマンドを execvp
                syscall(|| {
                    close(p.1).unwrap();
                    dup2(p.0, libc::STDIN_FILENO).unwrap();
                    close(p.0)
                })
                .unwrap();

                pids.insert(
                    child,
                    ProcInfo {
                        state: ProcState::Run,
                        pgid: getpgid(None).unwrap(),
                    },
                );
                match execvp(&filename, &args) {
                    Err(e) => {
                        eprintln!("{NAME}: Failed to exec: {e}");
                        exit(1);
                    }
                    Ok(_) => unreachable!(),
                }
            }
        }
    }
}

/// プロセスグループ ID を指定して fork & exec
/// pgid が 0 の場合は子プロセスのプロセス ID がプロセスグループ ID となる
///
/// - input が Some(fd) の場合は、標準入力を fd と設定
/// - output が Some(fd) の場合は、標準出力を fd と設定
fn fork_exec(
    pgid: Pid,
    cmds: &[model::ExternalCmd],
    pids: &mut HashMap<Pid, ProcInfo>,
) -> Result<Pid, DynError> {
    match syscall(|| unsafe { fork() })? {
        ForkResult::Parent { child } => {
            // 子プロセスのプロセスグループ ID を pgid に設定
            setpgid(child, pgid).unwrap();
            pids.insert(
                child,
                ProcInfo {
                    state: ProcState::Run,
                    pgid: child,
                },
            );

            Ok(child)
        }
        ForkResult::Child => {
            // 子プロセスのプロセスグループ ID を pgid に設定
            setpgid(Pid::from_raw(0), pgid).unwrap();

            do_pipeline(&mut VecDeque::from(cmds.to_vec()), pids);

            Ok(getpid())
        }
    }
}

type CmdResult<'a> = Result<Vec<model::Job>, DynError>;

/// コマンドをパース
fn parse_cmd(line: &str) -> CmdResult {
    parser::parse(line).map_err(Into::into)
}

/// ドロップ時にクロージャを呼び出す型
struct CleanUp<F>
where
    F: Fn(),
{
    f: F,
}
impl<F> Drop for CleanUp<F>
where
    F: Fn(),
{
    fn drop(&mut self) {
        (self.f)()
    }
}
