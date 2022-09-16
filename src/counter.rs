use std::{collections::HashMap, hash::Hash};

pub trait Counter<K, T> {
    fn add(&mut self, key: K) -> T;
    fn remove(&mut self, key: K) -> T;
    fn count(&self, key: &K) -> T;
}

impl<T: Eq + Hash> Counter<T, u64> for HashMap<T, u64> {
    fn add(&mut self, key: T) -> u64 {
        *self.entry(key).and_modify(|count| *count += 1).or_default()
    }

    fn remove(&mut self, key: T) -> u64 {
        *self
            .entry(key)
            .and_modify(|count| *count = count.saturating_sub(1))
            .or_default()
    }

    fn count(&self, key: &T) -> u64 {
        *self.get(&key).unwrap_or(&0)
    }
}
