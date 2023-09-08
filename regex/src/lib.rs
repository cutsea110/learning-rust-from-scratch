//! # 正規表現エンジン用クレート
//!
//! ## 利用例
//!
//! ```
//! use regex;
//! let expr = "a(bc)+|c(def)*"; // 正規表現と文字列をマッチング
//! let line = "cdefdefdef"; // マッチング対象の文字列
//! regex::do_matching(expr, line, true); // 深さ優先探索でマッチング
//! regex::print(expr); // 正規表現の AST と命令列を表示
//! ```
pub mod engine;
pub mod helper;

pub use engine::{do_matching, print};
