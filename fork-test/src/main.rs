//! パイプを使ったコマンドの連結
//! 参考: https://www.haya-programming.com/entry/2018/11/08/185349
use nix::{
    sys::wait::{waitpid, WaitPidFlag},
    unistd::{close, dup2, execvp, fork, pipe, ForkResult, Pid},
};
use std::ffi::CString;

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

fn dopipes(cmds: Vec<&Vec<&str>>) {
    if cmds.len() == 1 {
        // 最後なら端に execvp
        let filename = CString::new(cmds[0][0]).unwrap();
        let args = cmds[0]
            .iter()
            .map(|s| CString::new(*s).unwrap())
            .collect::<Vec<_>>();
        execvp(&filename, &args).unwrap();
    } else {
        // 端以外ならパイプを作って両端を再帰的に実行
        let p = pipe().unwrap();
        let pid = syscall(|| unsafe { fork() }).unwrap();
        match pid {
            ForkResult::Child => {
                // 子プロセスならパイプを stdout に dup2 して再帰
                syscall(|| {
                    close(p.0).unwrap();
                    dup2(p.1, 1).unwrap();
                    close(p.1)
                })
                .unwrap();

                dopipes(cmds[0..cmds.len() - 1].to_vec());
            }
            ForkResult::Parent { .. } => {
                // 親プロセスならパイプを stdin に dup2 して
                // 端のコマンドを execvp
                syscall(|| {
                    close(p.1).unwrap();
                    dup2(p.0, 0).unwrap();
                    close(p.0)
                })
                .unwrap();

                let i = cmds.len() - 1;
                let filename = CString::new(cmds[i][0]).unwrap();
                let args = cmds[i]
                    .iter()
                    .map(|s| CString::new(*s).unwrap())
                    .collect::<Vec<_>>();
                execvp(&filename, &args).unwrap();
            }
        }
    }
}

fn main() {
    let cmd1 = vec!["cat", "src/main.rs"];
    let cmd2 = vec!["head", "-n80"];
    let cmd3 = vec!["grep", "let"];
    let cmds = vec![&cmd1, &cmd2, &cmd3];

    let pid = syscall(|| unsafe { fork() }).unwrap();
    match pid {
        ForkResult::Child => {
            println!("child");
            dopipes(cmds);
        }
        ForkResult::Parent { child } => {
            println!("parent: child={}", child);
        }
    }
}
