mod combinator;

use combinator::*;

fn test() {
    satisfy(|x| x == "foo".to_string());
}
