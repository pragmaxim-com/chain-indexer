use std::collections::{HashMap, HashSet, VecDeque};

struct FifoCache<K, V> {
    max_size: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
    order_set: HashSet<K>,
}

impl<K: std::hash::Hash + Eq + Clone, V> FifoCache<K, V> {
    fn new(max_size: usize) -> Self {
        Self {
            max_size,
            map: HashMap::with_capacity(max_size),
            order: VecDeque::with_capacity(max_size),
            order_set: HashSet::with_capacity(max_size),
        }
    }

    fn insert(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            // Update existing entry, remove from order and reinsert
            self.order_set.remove(&key);
        } else if self.map.len() == self.max_size {
            // Evict the oldest entry
            if let Some(oldest_key) = self.order.pop_front() {
                self.map.remove(&oldest_key);
                self.order_set.remove(&oldest_key);
            }
        }

        self.order.push_back(key.clone());
        self.order_set.insert(key.clone());
        self.map.insert(key, value);
    }

    fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn last_entry(&self) -> Option<(&K, &V)> {
        if let Some(last_key) = self.order.back() {
            self.map.get_key_value(last_key)
        } else {
            None
        }
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> IntoIterator for FifoCache<K, V> {
    type Item = (K, V);
    type IntoIter = FifoCacheIntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        FifoCacheIntoIter { cache: self }
    }
}

struct FifoCacheIntoIter<K, V> {
    cache: FifoCache<K, V>,
}

impl<K: std::hash::Hash + Eq + Clone, V> Iterator for FifoCacheIntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(key) = self.cache.order.pop_front() {
            if let Some(value) = self.cache.map.remove(&key) {
                return Some((key, value));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insertion_and_size() {
        let mut cache = FifoCache::new(3);
        cache.insert(1, "a");
        cache.insert(2, "b");
        cache.insert(3, "c");

        assert_eq!(cache.len(), 3);
        assert!(cache.get(&1).is_some());
        assert!(cache.get(&2).is_some());
        assert!(cache.get(&3).is_some());

        cache.insert(4, "d");

        assert_eq!(cache.len(), 3);
        assert!(cache.get(&1).is_none());
        assert!(cache.get(&2).is_some());
        assert!(cache.get(&3).is_some());
        assert!(cache.get(&4).is_some());
    }

    #[test]
    fn test_replace_existing() {
        let mut cache = FifoCache::new(3);
        cache.insert(1, "a");
        cache.insert(2, "b");
        cache.insert(3, "c");

        cache.insert(2, "bb");

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&2), Some(&"bb"));
        assert!(cache.get(&1).is_some());
        assert!(cache.get(&3).is_some());

        cache.insert(4, "d");

        assert_eq!(cache.len(), 3);
        assert!(cache.get(&1).is_none());
        assert!(cache.get(&2).is_some());
        assert!(cache.get(&3).is_some());
        assert!(cache.get(&4).is_some());
    }

    #[test]
    fn test_iteration_order() {
        let mut cache = FifoCache::new(3);
        cache.insert(1, "a");
        cache.insert(2, "b");
        cache.insert(3, "c");

        let mut iter = cache.into_iter();
        assert_eq!(iter.next(), Some((1, "a")));
        assert_eq!(iter.next(), Some((2, "b")));
        assert_eq!(iter.next(), Some((3, "c")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_last_entry() {
        let mut cache = FifoCache::new(3);
        cache.insert(1, "a");
        cache.insert(2, "b");
        cache.insert(3, "c");

        assert_eq!(cache.last_entry(), Some((&3, &"c")));

        cache.insert(4, "d");
        assert_eq!(cache.last_entry(), Some((&4, &"d")));
    }
}
