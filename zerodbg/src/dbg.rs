use crate::helper::DynError;
use nix::{
    libc::user_regs_struct,
    sys::{
        personality::{self, Persona},
        ptrace,
        wait::{waitpid, WaitStatus},
    },
    unistd::{execvp, fork, ForkResult, Pid},
};
use std::ffi::{c_void, CString};

/// デバッガ内の情報
pub struct DbgInfo {
    pid: Pid,
    brk_addr: Option<*mut c_void>, // ブレークポイントのアドレス
    brk_val: i64,                  // ブレークポイントを設定したメモリの元の値
    filename: String,              // 実行ファイル名
}

/// デバッガ
/// ZDbg<Running> は子プロセスを実行中
/// ZDbg<NotRunning> は子プロセスは実行していない
pub struct ZDbg<T> {
    info: Box<DbgInfo>,
    _state: T,
}

/// デバッガの状態
pub struct Running; // 実行中
pub struct NotRunning; // 実行していない

/// デバッガの状態の列挙型表現
/// Exit の場合は終了
pub enum State {
    Running(ZDbg<Running>),
    NotRunning(ZDbg<NotRunning>),
    Exit,
}

/// Running と NotRunning で共通の実装
impl<T> ZDbg<T> {
    /// 共通のコマンドを実行
    fn do_cmd_common(&self, cmd: &[&str]) {
        match cmd[0] {
            "help" | "h" => do_help(),
            _ => (),
        }
    }

    /// ブレークポイントのアドレスを設定する関数
    /// 子プロセスのメモリ上には反映しない
    /// アドレス設定に成功した場合は true を返す
    fn set_break_addr(&mut self, cmd: &[&str]) -> bool {
        if self.info.brk_addr.is_some() {
            println!(
                "ブレークポイントは設定済みです : Addr = {:?}>>",
                self.info.brk_addr.unwrap()
            );
            false
        } else if let Some(addr) = get_break_addr(cmd) {
            self.info.brk_addr = Some(addr); // ブレークポイントのアドレスを設定
            true
        } else {
            false
        }
    }
}

/// NotRunning 時に呼び出し可能なメソッド
impl ZDbg<NotRunning> {
    pub fn new(filename: String) -> Self {
        Self {
            info: Box::new(DbgInfo {
                pid: Pid::from_raw(0),
                brk_addr: None,
                brk_val: 0,
                filename,
            }),
            _state: NotRunning,
        }
    }

    pub fn do_cmd(mut self, cmd: &[&str]) -> Result<State, DynError> {
        if cmd.is_empty() {
            return Ok(State::NotRunning(self));
        }

        match cmd[0] {
            "run" | "r" => return self.do_run(cmd),
            "break" | "b" => {
                self.do_break(cmd);
            }
            "exit" | "q" => return Ok(State::Exit),
            "continue" | "c" | "stepi" | "s" | "registers" | "regs" => {
                eprintln!("<<ターゲットを実行していません。 run で実行してください>>");
            }
            _ => self.do_cmd_common(cmd),
        }

        Ok(State::NotRunning(self))
    }

    /// ブレークポイントを設定
    fn do_break(&mut self, cmd: &[&str]) -> bool {
        self.set_break_addr(cmd)
    }

    /// 子プロセスを生成し、成功した場合は Running 状態に遷移
    fn do_run(mut self, cmd: &[&str]) -> Result<State, DynError> {
        // 子プロセスに渡すコマンドライン引数
        let args: Vec<CString> = cmd.iter().map(|s| CString::new(*s).unwrap()).collect();

        match unsafe { fork()? } {
            ForkResult::Child => {
                // ASLR の無効化
                // Linux ではセキュリティ上の理由から ASLR が有効になっている
                // デバッグ時には不便なので無効化する
                let p = personality::get().unwrap();
                personality::set(p | Persona::ADDR_NO_RANDOMIZE).unwrap();
                // 自身がデバッガによるトレース対象であることを指定
                // traceme を指定した後は exec すると即座にプロセスが停止するようになる
                ptrace::traceme().unwrap();

                // 子プロセスを実行
                execvp(&CString::new(self.info.filename.as_str()).unwrap(), &args).unwrap();
                unreachable!();
            }
            ForkResult::Parent { child } => match waitpid(child, None)? {
                // 子プロセスで traceme しているので子プロセスは停止もしくは終了するはず
                WaitStatus::Stopped(..) => {
                    println!("<<子プロセスの実行に成功しました : PID = {child}>>");
                    self.info.pid = child;
                    let mut dbg = ZDbg::<Running> {
                        info: self.info,
                        _state: Running,
                    };
                    // ブレークポイントはプロセスの実行中にしか行えないのでこの時点で設定
                    dbg.set_break()?;
                    // 子プロセスの実行を再開
                    dbg.do_continue()
                }
                WaitStatus::Exited(..) | WaitStatus::Signaled(..) => {
                    Err("子プロセスの実行に失敗しました".into())
                }
                _ => Err("子プロセスが不正な状態です".into()),
            },
        }
    }
}

/// Running 時に呼び出し可能なメソッド
impl ZDbg<Running> {
    pub fn do_cmd(mut self, cmd: &[&str]) -> Result<State, DynError> {
        if cmd.is_empty() {
            return Ok(State::Running(self));
        }

        match cmd[0] {
            "break" | "b" => self.do_break(cmd)?,
            "continue" | "c" => return self.do_continue(),
            "registers" | "regs" => {
                let regs = ptrace::getregs(self.info.pid)?;
                print_regs(&regs);
            }
            "stepi" | "s" => return self.do_stepi(),
            "run" | "r" => eprintln!("<<すでに実行中です>>"),
            "exit" | "q" => {
                self.do_exit()?;
                return Ok(State::Exit);
            }
            _ => self.do_cmd_common(cmd),
        }

        Ok(State::Running(self))
    }
    /// stepi を実行。機械語レベルで 1 行実行
    fn do_stepi(self) -> Result<State, DynError> {
        let regs = ptrace::getregs(self.info.pid)?;
        if Some((regs.rip) as *mut c_void) == self.info.brk_addr {
            // 次の実行先がブレークポイントのアドレスの場合、
            // 先に、 0xcc(int 3) に書き換えたメモリを元に戻してから実行する必要がある

            ptrace::write(
                self.info.pid,
                self.info.brk_addr.unwrap(),
                self.info.brk_val,
            )?;

            self.step_and_break()
        } else {
            ptrace::step(self.info.pid, None)?;
            self.wait_child()
        }
    }
    /// ブレークポイントで停止していた場合は
    /// 1 ステップ実行しブレークポイントを再設定
    fn step_and_break(mut self) -> Result<State, DynError> {
        let regs = ptrace::getregs(self.info.pid)?;
        if Some((regs.rip) as *mut c_void) == self.info.brk_addr {
            ptrace::step(self.info.pid, None)?; // 1 ステップ実行
            match waitpid(self.info.pid, None)? {
                WaitStatus::Exited(..) | WaitStatus::Signaled(..) => {
                    println!("<<子プロセスが終了しました>>");
                    return Ok(State::NotRunning(ZDbg::<NotRunning> {
                        info: self.info,
                        _state: NotRunning,
                    }));
                }
                _ => (),
            }
            self.set_break()?; // ブレークポイントを再設定
        }

        Ok(State::Running(self))
    }
    /// ブレークポイントを実際に設定
    /// つまり、該当アドレスのメモリを 0xcc(int 3) に設定
    fn set_break(&mut self) -> Result<(), DynError> {
        let addr = if let Some(addr) = self.info.brk_addr {
            addr
        } else {
            return Ok(());
        };

        // ブレークするアドレスにあるメモリ上の値を取得
        // メモリの値は i64(8bytes) で返される
        let val = match ptrace::read(self.info.pid, addr) {
            Ok(val) => val,
            Err(e) => {
                eprintln!("<<ptrace::read に失敗 : {e}, addr = {addr:?}>>");
                return Ok(());
            }
        };

        // メモリ上の値を表示する補助関数
        // read で得られた値と 0xcc で書き換えた値をわかりやすく表示する
        fn print_val(addr: usize, val: i64) {
            print!("{addr:x}");
            for n in (0..8).map(|n| ((val >> (n * 8)) & 0xff) as u8) {
                print!(" {n:x}");
            }
        }

        println!("<<以下のようにメモリを書き換えます>>");
        print!("<<before: "); // 元の値を表示
        print_val(addr as usize, val);
        println!(">>");

        // "int 3" に設定する
        let val_int3 = (val & !0xff) | 0xcc;
        print!("<<after : "); // 変更後の値を表示
        print_val(addr as usize, val_int3);
        println!(">>");

        // "int 3" をメモリに書き込み
        // as *mut c_void と型変換しているのは、C の ptrace が引数にポインタをとるため
        match ptrace::write(self.info.pid, addr, val_int3) {
            Ok(_) => {
                self.info.brk_addr = Some(addr);
                self.info.brk_val = val; // 元の値を保持
            }
            Err(e) => {
                eprintln!("<<ptrace::write に失敗 : {e}, addr = {addr:p}>>");
            }
        }

        Ok(())
    }
    /// break を実行
    fn do_break(&mut self, cmd: &[&str]) -> Result<(), DynError> {
        if self.set_break_addr(cmd) {
            self.set_break()?;
        }
        Ok(())
    }
    /// continue を実行
    fn do_continue(self) -> Result<State, DynError> {
        // ブレークポイントで停止していた場合は 1 ステップ実行後に再設定
        match self.step_and_break()? {
            State::Running(r) => {
                // 実行再開
                ptrace::cont(r.info.pid, None)?;
                r.wait_child()
            }
            n => Ok(n),
        }
    }
    /// 子プロセスを wait 。子プロセスが終了した場合は NotRunning 状態に遷移
    fn wait_child(self) -> Result<State, DynError> {
        match waitpid(self.info.pid, None)? {
            WaitStatus::Exited(..) | WaitStatus::Signaled(..) => {
                println!("<<子プロセスが終了しました>>");
                let not_run = ZDbg::<NotRunning> {
                    info: self.info,
                    _state: NotRunning,
                };
                Ok(State::NotRunning(not_run))
            }
            WaitStatus::Stopped(..) => {
                let mut regs = ptrace::getregs(self.info.pid)?;
                if Some((regs.rip - 1) as *mut c_void) == self.info.brk_addr {
                    // 書き換えたメモリを元の値に戻す
                    ptrace::write(
                        self.info.pid,
                        self.info.brk_addr.unwrap(),
                        self.info.brk_val,
                    )?;

                    // ブレークポイントで停止したアドレスから 1 つ戻す
                    regs.rip -= 1;
                    ptrace::setregs(self.info.pid, regs)?;
                }
                println!("<<子プロセスが停止しました : PC = {:#x}>>", regs.rip);

                Ok(State::Running(self))
            }
            _ => Err("waitpid の返り値が不正です".into()),
        }
    }
    /// exit を実行。実行中のプロセスは kill
    fn do_exit(self) -> Result<(), DynError> {
        loop {
            ptrace::kill(self.info.pid)?;
            match waitpid(self.info.pid, None)? {
                WaitStatus::Exited(..) | WaitStatus::Signaled(..) => return Ok(()),
                _ => (),
            }
        }
    }
}

/// ヘルプを表示
fn do_help() {
    println!(
        r#"コマンド一覧 (括弧内は省略記法)
break 0x8000  : ブレークポイントを 0x8000 番地に設定 (b 0x8000)
run           : プログラムを実行 (r)
continue      : プログラムを再開 (c)
stepi         : 機械語レベルで 1 ステップ実行 (s)
registers     : レジスタを表示 (regs)
exit          : 終了 (q)
help          : このヘルプを表示 (h)"#
    );
}

/// コマンドからブレークポイントを計算
fn get_break_addr(cmd: &[&str]) -> Option<*mut c_void> {
    if cmd.len() < 2 {
        eprintln!("アドレスを指定してください\n 例 : break 0x8000>>");
        return None;
    }

    let addr_str = cmd[1];
    if &addr_str[0..2] != "0x" {
        eprintln!("<<アドレスは 16 進数でのみ指定可能です\n 例 : break 0x8000>>");
        return None;
    }

    let addr = match usize::from_str_radix(&addr_str[2..], 16) {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("<<アドレス変換エラー : {e}>>");
            return None;
        }
    } as *mut c_void;

    Some(addr)
}

/// レジスタを表示
fn print_regs(regs: &user_regs_struct) {
    println!(
        r#"RIP: {:#016x}, RSP: {:#016x}, RBP: {:#016x}
RAX: {:#016x}, RBX: {:#016x}, RCX: {:#016x}
RDX: {:#016x}, RSI: {:#016x}, RDI: {:#016x}
 R8: {:#016x},  R9: {:#016x}, R10: {:#016x}
R11: {:#016x}, R12: {:#016x}, R13: {:#016x}
R14: {:#016x}, R15: {:#016x}"#,
        regs.rip,
        regs.rsp,
        regs.rbp,
        regs.rax,
        regs.rbx,
        regs.rcx,
        regs.rdx,
        regs.rsi,
        regs.rdi,
        regs.r8,
        regs.r9,
        regs.r10,
        regs.r11,
        regs.r12,
        regs.r13,
        regs.r14,
        regs.r15
    );
}
