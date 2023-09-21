use num::{BigUint, FromPrimitive, One, Zero};

const M: usize = 4;
const N: usize = 4;

fn main() {
    let m = M;
    let n = BigUint::from_usize(N).unwrap();
    let a = ackerman(m, n.clone());
    println!("ackermann({M}, {N} = {a}");
}

fn ackerman(m: usize, n: BigUint) -> BigUint {
    todo!()
}
