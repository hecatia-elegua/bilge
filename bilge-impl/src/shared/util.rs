use std::collections::HashSet;
use std::hash::Hash;

/// an alternative to `HashSet<T>` where `T` is not `Hash + Eq`. 
/// allows hashing by `hashing_func` instead
pub struct DedupedVec<T, H: Hash + Eq> {
    vec: Vec<T>,
    hashing_func: fn(&T) -> H,
    hashes: HashSet<H>,
}

impl<T, H: Hash + Eq> DedupedVec<T, H> {
    pub fn new(hashing_func: fn(&T) -> H) -> Self {
        DedupedVec {
            vec: Vec::new(),
            hashing_func,
            hashes: HashSet::new(),
        }
    }
    
    pub fn push(&mut self, elem: T) -> bool {
        let hash = (self.hashing_func)(&elem);
        let not_seen_yet = self.hashes.insert(hash);
        
        if not_seen_yet {
            self.vec.push(elem);
            true
        } else {
            false
        }
    }
}

impl<T, H: Hash + Eq> IntoIterator for DedupedVec<T, H> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
    }
}