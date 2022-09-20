use dashmap::DashMap;
use std::hash::Hash;

pub trait Counter<K, T> {
    fn add(&self, key: K) -> T;
    fn remove(&self, key: K) -> T;
    fn count(&self, key: &K) -> T;
}

impl<T: Eq + Hash> Counter<T, u64> for DashMap<T, u64> {
    fn add(&self, key: T) -> u64 {
        *self.entry(key).and_modify(|count| *count += 1).or_default()
    }

    fn remove(&self, key: T) -> u64 {
        *self
            .entry(key)
            .and_modify(|count| *count = count.saturating_sub(1))
            .or_default()
    }

    fn count(&self, key: &T) -> u64 {
        self.view(key, |_k, v| *v).unwrap_or(0)
    }
}
