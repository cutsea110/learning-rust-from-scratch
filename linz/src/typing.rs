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
