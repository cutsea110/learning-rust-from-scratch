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
