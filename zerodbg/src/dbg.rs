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
    // TODO
}

/// NotRunning 時に呼び出し可能なメソッド
impl ZDbg<NotRunning> {
    // TODO
}

/// Running 時に呼び出し可能なメソッド
impl ZDbg<Running> {
    fn do_stepi(self) -> Result<State, DynError> {
        todo!()
    }
}

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
