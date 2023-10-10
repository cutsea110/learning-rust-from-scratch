use crate::helper::*;
use crate::lang;
use std::{borrow::Cow, cmp::Ordering, collections::BTreeMap, mem};

type VarToType = BTreeMap<String, Option<lang::TypeExpr>>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct TypeEnvStack {
    vars: BTreeMap<usize, VarToType>,
}

impl TypeEnvStack {
    fn new() -> Self {
        Self {
            vars: BTreeMap::new(),
        }
    }

    /// 型環境を push
    fn push(&mut self, depth: usize) {
        self.vars.insert(depth, BTreeMap::new());
    }

    /// 型環境を pop
    fn pop(&mut self, depth: usize) -> Option<VarToType> {
        self.vars.remove(&depth)
    }

    /// スタックの最も上にある型環境に変数と型を追加
    fn insert(&mut self, key: String, value: lang::TypeExpr) {
        if let Some(last) = self.vars.iter_mut().next_back() {
            last.1.insert(key, Some(value));
        }
    }

    /// スタックを上から底に向かって探索し、最初に見つかった変数の型を返す
    fn get_mut(&mut self, key: &str) -> Option<(usize, &mut Option<lang::TypeExpr>)> {
        for (depth, env) in self.vars.iter_mut().rev() {
            if let Some(ty) = env.get_mut(key) {
                return Some((*depth, ty));
            }
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeEnv {
    env_lin: TypeEnvStack,
    env_un: TypeEnvStack,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            env_lin: TypeEnvStack::new(),
            env_un: TypeEnvStack::new(),
        }
    }

    /// 型環境を push
    fn push(&mut self, depth: usize) {
        self.env_lin.push(depth);
        self.env_un.push(depth);
    }

    /// 型環境を pop
    fn pop(&mut self, depth: usize) -> (Option<VarToType>, Option<VarToType>) {
        let t1 = self.env_lin.pop(depth);
        let t2 = self.env_un.pop(depth);

        (t1, t2)
    }

    /// 型環境へ変数と型を追加
    fn insert(&mut self, key: String, value: lang::TypeExpr) {
        match value.qual {
            lang::Qual::Lin => self.env_lin.insert(key, value),
            lang::Qual::Un => self.env_un.insert(key, value),
        }
    }

    /// lin と un の型環境から get_mut を呼び出し depth が大きい方を返す
    fn get_mut(&mut self, key: &str) -> Option<&mut Option<lang::TypeExpr>> {
        match (self.env_lin.get_mut(key), self.env_un.get_mut(key)) {
            (Some((d1, t1)), Some((d2, t2))) => match d1.cmp(&d2) {
                Ordering::Less => Some(t2),
                Ordering::Greater => Some(t1),
                Ordering::Equal => panic!("invalid type environment"),
            },
            (Some((_, t1)), None) => Some(t1),
            (None, Some((_, t2))) => Some(t2),
            _ => None,
        }
    }
}

type TResult<'a> = Result<lang::TypeExpr, Cow<'a, str>>;

/// 型付け関数
/// 式を受け取り, 型を返す
pub fn typing<'a>(expr: &lang::Expr, env: &mut TypeEnv, depth: usize) -> TResult<'a> {
    match expr {
        lang::Expr::App(e) => typing_app(e, env, depth),
        lang::Expr::QVal(e) => typing_qval(e, env, depth),
        lang::Expr::Free(e) => typing_free(e, env, depth),
        lang::Expr::If(e) => typing_if(e, env, depth),
        lang::Expr::Split(e) => typing_split(e, env, depth),
        lang::Expr::Var(e) => typing_var(e, env, depth),
        lang::Expr::Let(e) => typing_let(e, env, depth),
    }
}

/// 関数適用の型付け
fn typing_app<'a>(expr: &lang::AppExpr, env: &mut TypeEnv, depth: usize) -> TResult<'a> {
    // 関数部分
    let t1 = typing(&expr.expr1, env, depth)?;
    let t_arg;
    let t_ret;
    match t1.prim {
        lang::PrimType::Arrow(a, b) => {
            t_arg = a; // 引数の型
            t_ret = b; // 返り値の型
        }
        _ => return Err("関数型でない".into()),
    }

    // 引数部分
    let t2 = typing(&expr.expr2, env, depth)?;

    // 引数の型が一致しているかチェック
    if *t_arg == t2 {
        Ok(*t_ret)
    } else {
        Err("関数適用時における引数の型が異なる".into())
    }
}

/// 修飾子付き値の型付け
fn typing_qval<'a>(expr: &lang::QValExpr, env: &mut TypeEnv, depth: usize) -> TResult<'a> {
    // プリミティブ型を計算
    let p = match &expr.val {
        lang::ValExpr::Bool(_) => lang::PrimType::Bool,
        lang::ValExpr::Pair(e1, e2) => {
            // 式 e1 と e2 を typing により型付け
            let t1 = typing(e1, env, depth)?;
            let t2 = typing(e2, env, depth)?;

            // un 型のペアは lin 型の値を内包できないという制約がある
            if expr.qual == lang::Qual::Un
                && (t1.qual == lang::Qual::Lin || t2.qual == lang::Qual::Lin)
            {
                return Err("un型のペア内でlin型を利用している".into());
            }

            // ペア型を返す
            lang::PrimType::Pair(Box::new(t1), Box::new(t2))
        }
        lang::ValExpr::Fun(e) => {
            // 関数の型付け

            // un 型の関数の場合, この関数の外側で定義された lin 型の変数は利用できない
            // そのため lin 用の型環境を空にする
            // ただし, あとで環境を復元する必要があるので退避しておく
            // これが lin と un で型環境を別に用意し, BTreeMap でスタックを実装した理由である
            let env_prev = if expr.qual == lang::Qual::Un {
                Some(mem::take(&mut env.env_lin))
            } else {
                None
            };

            // 型環境のスタックをインクリメントする
            // スタックのプッシュには depth が必要なので忘れずにインクリメントする
            let mut depth = depth;
            safe_add(&mut depth, &1, || "変数スコープのネストが深すぎる")?;
            env.push(depth);
            env.insert(e.var.clone(), e.ty.clone());

            // 関数中の式を型付け
            let t = typing(&e.expr, env, depth)?;

            // スタックを pop し, pop した型環境の中に lin 型が含まれていた場合は
            // 消費されなかったということなのでエラー
            // このように型環境をスタックにすることで変数のスコープが表現されている
            // また get_mut をスタック上位から下位に向かって検索するようにしたことでシャドウイングを実現
            let (elin, _) = env.pop(depth);
            for (k, v) in elin.unwrap().iter() {
                if v.is_some() {
                    return Err(format!(r#"関数定義内でlin型の変数"{k}"を消費していない"#).into());
                }
            }

            // 上で退避していた lin 用の型環境を復元
            if let Some(ep) = env_prev {
                env.env_lin = ep;
            }

            // 関数型を返す
            lang::PrimType::Arrow(Box::new(e.ty.clone()), Box::new(t))
        }
    };

    // 修飾子付き型を返す
    Ok(lang::TypeExpr {
        qual: expr.qual,
        prim: p,
    })
}

/// free 式の型付け
fn typing_free<'a>(expr: &lang::FreeExpr, env: &mut TypeEnv, depth: usize) -> TResult<'a> {
    if let Some((_, t)) = env.env_lin.get_mut(&expr.var) {
        if t.is_some() {
            *t = None;
            return typing(&expr.expr, env, depth);
        }
    }
    Err(format!(
        r#"すでにfreeしたか、lin型ではない変数"{}"をfreeしている"#,
        expr.var
    )
    .into())
}

/// if 式の型付け
fn typing_if<'a>(expr: &lang::IfExpr, env: &mut TypeEnv, depth: usize) -> TResult<'a> {
    let t1 = typing(&expr.cond_expr, env, depth)?;
    //条件式の型は bool
    if t1.prim != lang::PrimType::Bool {
        return Err("ifの条件式がboolでない".into());
    }

    // then と else で別々の式を同じ型環境で検査するため
    // 型環境を clone してからそれぞれの式の型付けを行う
    let mut e = env.clone();
    let t2 = typing(&expr.then_expr, &mut e, depth)?;
    let t3 = typing(&expr.else_expr, env, depth)?;

    // then と else 式の型は同じで
    // then と else 式の評価後の型環境が同じかチェック
    if t2 != t3 || e != *env {
        return Err("if式のthen節とelse節の式の型が異なる".into());
    }

    Ok(t2)
}

/// split 式の型付け
fn typing_split<'a>(expr: &lang::SplitExpr, env: &mut TypeEnv, depth: usize) -> TResult<'a> {
    // 同じ変数名は使えない
    if expr.left == expr.right {
        return Err("splitの変数名が同じ".into());
    }

    let t1 = typing(&expr.expr, env, depth)?;
    let mut depth = depth;
    safe_add(&mut depth, &1, || "変数スコープのネストが深すぎる")?;

    match t1.prim {
        lang::PrimType::Pair(p1, p2) => {
            env.push(depth);
            // ローカル変数の型を追加
            env.insert(expr.left.clone(), *p1);
            env.insert(expr.right.clone(), *p2);
        }
        _ => {
            return Err("splitの引数がペア型でない".into());
        }
    }

    let ret = typing(&expr.body, env, depth);

    // 型環境をポップする(ローカル変数を削除)
    let (elin, _) = env.pop(depth);

    // ポップした型環境の中に lin 型の変数が残っていないかをチェック
    // 残っていたら消費していない lin 型の値があるということなのでエラー
    for (k, v) in elin.unwrap().iter() {
        if v.is_some() {
            return Err(format!(r#"splitの式内でlin型の変数"{k}"を消費していない"#).into());
        }
    }

    ret
}

/// 変数の型付け
fn typing_var<'a>(expr: &str, env: &mut TypeEnv, depth: usize) -> TResult<'a> {
    let ret = env.get_mut(expr);
    if let Some(it) = ret {
        // 定義されている
        if let Some(t) = it {
            // 消費されていない
            match t.qual {
                lang::Qual::Lin => {
                    // lin 型
                    let eret = t.clone();
                    *it = None; // lin を消費
                    return Ok(eret);
                }
                lang::Qual::Un => {
                    return Ok(t.clone());
                }
            }
        }
    }

    Err(format!(
        r#""{}"という変数は定義されていないか、利用済みか、キャプチャできない"#,
        expr
    )
    .into())
}

/// let 式の型付け
fn typing_let<'a>(expr: &lang::LetExpr, env: &mut TypeEnv, depth: usize) -> TResult<'a> {
    // 変数束縛
    let t1 = typing(&expr.expr1, env, depth)?;
    // 束縛変数の型をチェック
    if t1 != expr.ty {
        return Err(format!(r#"変数"{}"の型が異なる"#, expr.var).into());
    }

    // 関数内
    let mut depth = depth;
    safe_add(&mut depth, &1, || "変数スコープのネストが深すぎる")?;
    env.push(depth);
    env.insert(expr.var.clone(), t1); // 変数の型を insert
    let t2 = typing(&expr.expr2, env, depth)?;

    // lin 型の変数を消費しているかチェック
    let (elin, _) = env.pop(depth);
    for (k, v) in elin.unwrap().iter() {
        if v.is_some() {
            return Err(format!(r#"let式内でlin型の変数"{k}"を消費していない"#).into());
        }
    }

    Ok(t2)
}
