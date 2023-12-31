use nix::{
    sys::signal::{kill, Signal},
    unistd::getpid,
};
use std::arch::asm;

fn main() {
    println!("int 3");
    unsafe { asm!("int 3") };

    println!("kill -SIGTRAP");
    let pid = getpid();
    kill(pid, Signal::SIGTRAP).unwrap();

    for i in 0..3 {
        unsafe { asm!("nop") };
        println!("i = {i}");
    }
}
