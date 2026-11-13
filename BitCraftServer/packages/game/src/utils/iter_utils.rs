use std::collections::HashMap;
use std::hash::Hash;

#[allow(dead_code)]
pub(crate) trait GroupBy: Iterator {
    fn group_by<K, V, FKey, FVal>(self, f_key: FKey, f_val: FVal) -> HashMap<K, Vec<V>>
    where
        Self: Sized,
        K: Ord + Eq + Hash,
        FKey: Fn(&Self::Item) -> K,
        FVal: Fn(&Self::Item) -> V,
    {
        let mut r = HashMap::new();
        let mut col: Vec<Self::Item> = self.collect();
        if col.is_empty() {
            return r;
        }
        col.sort_by_key(&f_key);
        let mut cur_key = f_key(&col[0]);
        let mut cur_vec: Vec<V> = Vec::new();
        for kv in col {
            let k = f_key(&kv);
            let v = f_val(&kv);
            if k != cur_key {
                r.insert(cur_key, cur_vec);
                cur_key = k;
                cur_vec = Vec::new();
            }
            cur_vec.push(v);
        }
        r.insert(cur_key, cur_vec);
        r
    }
}

impl<I: Iterator> GroupBy for I {}

pub(crate) trait GroupByAndCount: Iterator {
    #[allow(dead_code)]
    fn group_by_and_count<K, FKey>(self, f_key: FKey) -> HashMap<K, usize>
    where
        Self: Sized,
        K: Ord + Eq + Hash,
        FKey: Fn(&Self::Item) -> K,
    {
        let mut r = HashMap::new();
        let mut col: Vec<Self::Item> = self.collect();
        if col.is_empty() {
            return r;
        }
        col.sort_by_key(&f_key);
        let mut cur_key = f_key(&col[0]);
        let mut cur_count = 0;
        for kv in col {
            let k = f_key(&kv);
            if k != cur_key {
                r.insert(cur_key, cur_count);
                cur_key = k;
                cur_count = 0;
            }
            cur_count += 1;
        }
        r.insert(cur_key, cur_count);
        r
    }
}

impl<I: Iterator> GroupByAndCount for I {}
