use std::fmt::Display;

pub mod codegen;
pub mod evaluator;
pub mod parser;
use crate::helper::DynError;

#[derive(Debug)]
pub enum Instruction {
    Char(char),
    Match,
    Jump(usize),
    Split(usize, usize),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Char(c) => write!(f, "char {c}"),
            Instruction::Match => write!(f, "match"),
            Instruction::Jump(addr) => write!(f, "jump {addr:>04}"),
            Instruction::Split(addr1, addr2) => write!(f, "split {addr1:>04} {addr2:>04}"),
        }
    }
}

/// 正規表現をパースしてコード生成し、
/// ASTと命令列を標準出力に表示。
///
/// # 利用例
///
/// ```
/// use regex;
/// regex::print("abc|(de|cd)+");
/// ```
///
/// # 返り値
///
/// 入力された正規表現にエラーがあったり、内部的な実装エラーがある場合はErrを返す。
pub fn print(expr: &str) -> Result<(), DynError> {
    println!("expr: {expr}");
    let ast = parser::parse(expr)?;
    println!("AST: {ast:?}");

    println!();
    println!("code:");
    let code = codegen::get_code(&ast)?;
    for (n, c) in code.iter().enumerate() {
        println!("{n:>04}: {c}");
    }

    Ok(())
}

/// 正規表現と文字列をマッチング。
///
/// # 利用例
///
/// ```
/// use regex;
/// regex::do_matching("abc|(de|cd)+", "decddede", true);
/// ```
///
/// # 引数
///
/// expr に正規表現、 line にマッチング対象の文字列を指定。
/// is_depth に true を指定すると深さ優先探索、 false を指定すると幅優先探索でマッチングを行う。
///
///
/// # 戻り値
///
/// エラーなく実行でき、かつマッチングに **成功** した場合は Ok(true) を返し、
/// エラーなく実行でき、かつマッチングに **失敗** した場合は Ok(false) を返す。
///
/// 入力された正規表現にエラーがあったり、内部的な実装エラーがある場合は Err を返す。
pub fn do_matching(expr: &str, line: &str, is_depth: bool) -> Result<bool, DynError> {
    let ast = parser::parse(expr)?;
    let code = codegen::get_code(&ast)?;
    let line = line.chars().collect::<Vec<char>>();
    Ok(evaluator::eval(&code, &line, is_depth)?)
}
