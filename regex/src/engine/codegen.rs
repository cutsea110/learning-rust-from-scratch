use super::{parser::AST, Instruction};
use crate::helper::safe_add;
use std::{
    error::Error,
    fmt::{self, Display},
};

/// コード生成エラーを表す型
#[derive(Debug)]
pub enum CodeGenError {
    PCoverFlow,
    FailStar,
    FailOr,
    FailQuestion,
}

impl Display for CodeGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodeGenError: {self:?}")
    }
}

impl Error for CodeGenError {}

/// コード生成器。
#[derive(Default, Debug)]
struct Generator {
    pc: usize,
    insts: Vec<Instruction>,
}

pub fn get_code(ast: &AST) -> Result<Vec<Instruction>, CodeGenError> {
    let mut code_gen = Generator::default();
    code_gen.gen_code(ast)?;
    Ok(code_gen.insts)
}

impl Generator {
    /// コード生成を行う関数。
    fn gen_code(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        self.gen_expr(ast)?;
        self.inc_pc()?;
        self.insts.push(Instruction::Match);
        Ok(())
    }

    /// プログラムカウンタをインクリメント。
    fn inc_pc(&mut self) -> Result<(), CodeGenError> {
        safe_add(&mut self.pc, &1, || CodeGenError::PCoverFlow)
    }

    /// AST をパターン分けし、コード生成を行う関数。
    fn gen_expr(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        match ast {
            AST::Char(c) => self.gen_char(*c)?,
            AST::Or(e1, e2) => self.gen_or(e1, e2)?,
            AST::Plus(e) => self.gen_plus(e)?,
            AST::Star(e) => self.gen_star(e)?,
            AST::Question(e) => self.gen_question(e)?,
            AST::Seq(es) => self.gen_seq(es)?,
        }

        Ok(())
    }

    /// char 命令生成関数。
    fn gen_char(&mut self, c: char) -> Result<(), CodeGenError> {
        self.insts.push(Instruction::Char(c));
        self.inc_pc()?;
        Ok(())
    }

    /// Or 演算子のコード生成。
    ///
    /// 以下のようなコードを生成する。
    ///
    /// ```text
    ///     split L1, L2
    /// L1: e1 のコード
    ///     jump L3
    /// L2: e2 のコード
    /// L3:
    /// ```
    fn gen_or(&mut self, e1: &AST, e2: &AST) -> Result<(), CodeGenError> {
        // split L1, L2
        let split_addr = self.pc;
        self.inc_pc()?;
        let split = Instruction::Split(self.pc, 0); // L1 = self.pc, L2 を仮に 0 としておく
        self.insts.push(split);

        // L1: e1 のコード
        self.gen_expr(e1)?;

        // jump L3
        let jmp_addr = self.pc;
        self.insts.push(Instruction::Jump(0)); // L3 を仮に 0 と設定しておく
        self.inc_pc()?;

        // L2 の値を設定
        if let Some(Instruction::Split(_, l2)) = self.insts.get_mut(split_addr) {
            *l2 = self.pc;
        } else {
            return Err(CodeGenError::FailOr);
        }

        // L2: e2 のコード
        self.gen_expr(e2)?;

        // L3 の値を設定
        if let Some(Instruction::Jump(l3)) = self.insts.get_mut(jmp_addr) {
            *l3 = self.pc;
        } else {
            return Err(CodeGenError::FailOr);
        }

        Ok(())
    }

    /// Plus 演算子のコード生成。
    ///
    /// 以下のようなコードを生成する。
    ///
    /// ```text
    /// L1: e1 のコード
    ///     split L1, L2
    /// L2:
    /// ```
    fn gen_plus(&mut self, e1: &AST) -> Result<(), CodeGenError> {
        // L1: e のコード
        let l1 = self.pc;
        self.gen_expr(e1)?;

        // split L1, L2
        self.inc_pc()?;
        let split = Instruction::Split(l1, self.pc); // L2 = self.pc
        self.insts.push(split);

        Ok(())
    }

    /// Star 演算子のコード生成。
    ///
    /// 以下のようなコードを生成する。
    ///
    /// ```text
    /// L1: split L2, L3
    /// L2: e1 のコード
    ///	    jump L1
    /// L3:
    /// ```
    fn gen_star(&mut self, e1: &AST) -> Result<(), CodeGenError> {
        // L1: split L2, L3
        let l1 = self.pc;
        self.inc_pc()?;
        self.insts.push(Instruction::Split(self.pc, 0)); // L2 = self.pc, L3 を仮に 0 としておく
        self.gen_expr(e1)?;

        // jump L1
        self.insts.push(Instruction::Jump(l1));
        self.inc_pc()?;

        // L3 の値を設定
        if let Some(Instruction::Split(_, l3)) = self.insts.get_mut(l1) {
            *l3 = self.pc;
        } else {
            return Err(CodeGenError::FailStar);
        }

        Ok(())
    }

    /// Question 演算子のコード生成。
    ///
    /// 以下のようなコードを生成する。
    ///
    /// ```text
    ///     split L1, L2
    /// L1: e1 のコード
    /// L2:
    /// ```
    fn gen_question(&mut self, e1: &AST) -> Result<(), CodeGenError> {
        // split L1, L2
        let split_addr = self.pc;
        self.inc_pc()?;
        let split = Instruction::Split(self.pc, 0); // L1 = self.pc, L2 を仮に 0 としておく
        self.insts.push(split);

        // L1: e のコード
        self.gen_expr(e1)?;

        // L2 の値を設定
        if let Some(Instruction::Split(_, l2)) = self.insts.get_mut(split_addr) {
            *l2 = self.pc;
        } else {
            return Err(CodeGenError::FailQuestion);
        }

        Ok(())
    }

    /// 連続する AST のコード生成。
    fn gen_seq(&mut self, exprs: &[AST]) -> Result<(), CodeGenError> {
        for e in exprs {
            self.gen_expr(e)?;
        }

        Ok(())
    }
}
