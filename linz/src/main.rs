pub use parser_combinator;
use std::{env, fs};

mod helper;
mod lang;
mod parser;
mod typing;

fn main() -> Result<(), helper::DynError> {
    // コマンドライン引数の検査
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("以下のようにファイル名を指定して実行してください\ncargo run codes/ex1.lin");
        return Err("引数が不足".into());
    }

    // ファイル読み込み
    let content = fs::read_to_string(&args[1])?;

    // パース
    let ast = parser::parse_expr(&content);
    // println!("AST:\n{ast:#?}");
    match ast {
        Ok((_, expr)) => {
            let mut ctx = typing::TypeEnv::new();
            println!("式:\n{content}");

            // 型付け
            let a = typing::typing(&expr, &mut ctx, 0)?;
            println!("の型は\n{a}\nです。");
        }
        Err(e) => {
            // TODO: エラーの位置を表示する
            let msg = format!("{e}");
            eprintln!("パースエラー:\n{msg}");
            return Err(msg.into());
        }
    }

    Ok(())
}
