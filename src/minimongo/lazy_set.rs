//惰性补集计算机制

use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use crate::minimongo::query::ConditionOperation;

#[derive(Debug, Serialize, Deserialize)]
pub struct LazySet {
    pub(crate) ids: BTreeSet<u32>,
    is_complement: bool, // 是否是补集
}

pub const MAX_FULL_LEN: u32 = 1000;

impl LazySet {
    // 创建普通集合
    pub(crate) fn new(ids: BTreeSet<u32>) -> Self {
        Self {
            ids,
            is_complement: false,
        }
    }

    // 创建补集
    pub(crate) fn complement(ids: BTreeSet<u32>) -> Self {
        Self {
            ids,
            is_complement: true,
        }
    }

    // 计算补集（全集减去当前集合）
    pub(crate) fn evaluate(&self) -> BTreeSet<u32> {
        if self.is_complement {
            BTreeSet::new()
        } else {
            self.ids.clone()
        }
    }

    pub(crate) fn evaluate_if<F1, F2>(self, get_full_set: F1, get_set_len: F2) -> BTreeSet<u32>
    where
        F1: Fn() -> BTreeSet<u32>,
        F2: Fn() -> u32,
    {
        if self.is_complement {
            if get_set_len() < MAX_FULL_LEN {
                let full_set = get_full_set();
                full_set.difference(&self.ids).cloned().collect()
            } else {
                BTreeSet::new()
            }
        } else {
            self.ids
        }
    }

    // 合并两个惰性集合（支持 AND 和 OR）
    pub(crate) fn merge(&self, other: &Self, operation: ConditionOperation) -> Self {
        match operation {
            ConditionOperation::AND(_) => {
                if self.is_complement && other.is_complement {
                    // A' AND B' -> (A OR B)'
                    let mut union = self.ids.clone();
                    union.extend(&other.ids);
                    Self::complement(union)
                } else if self.is_complement {
                    // A' AND B -> B - A
                    let difference = other.ids.difference(&self.ids).cloned().collect();
                    Self::new(difference)
                } else if other.is_complement {
                    // A AND B' -> A - B
                    let difference = self.ids.difference(&other.ids).cloned().collect();
                    Self::new(difference)
                } else {
                    // A AND B -> A ∩ B
                    let intersection = self.ids.intersection(&other.ids).cloned().collect();
                    Self::new(intersection)
                }
            }
            ConditionOperation::OR(_) => {
                if self.is_complement && other.is_complement {
                    // A' OR B' -> (A ∩ B)'
                    let intersection = self.ids.intersection(&other.ids).cloned().collect();
                    Self::complement(intersection)
                } else if self.is_complement {
                    // A' OR B -> A' ∪ B = 全集 - (A ∩ B)'
                    let difference = other.ids.difference(&self.ids).cloned().collect();
                    Self::complement(difference)
                } else if other.is_complement {
                    // A OR B' -> A ∪ B'
                    let difference = self.ids.difference(&other.ids).cloned().collect();
                    Self::complement(difference)
                } else {
                    // A OR B -> A ∪ B
                    let union = self.ids.union(&other.ids).cloned().collect();
                    Self::new(union)
                }
            }
            _ => panic!("Invalid operation for merge"),
        }
    }
}