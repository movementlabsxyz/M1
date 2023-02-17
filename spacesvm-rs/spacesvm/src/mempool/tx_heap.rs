use std::collections::{HashMap, VecDeque};

use avalanche_types::ids;

use crate::chain::tx::tx::Transaction;

/// In memory representation of mempool data.
#[derive(Debug)]
pub struct TxHeap {
    pub is_min: bool,
    pub items: VecDeque<Entry>,
    pub lookup: HashMap<ids::Id, Entry>,
}

/// Object representing a transaction entry stored in mempool.
#[derive(Debug, Default, Clone)]
pub struct Entry {
    pub id: ids::Id,
    pub tx: Option<Transaction>,
    pub index: usize,
}

impl TxHeap {
    pub fn new(max_size: usize, is_min: bool) -> Self {
        Self {
            is_min,
            items: VecDeque::with_capacity(max_size),
            lookup: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.len() == 0
    }

    pub fn swap(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        self.items[i].index = i;
        self.items[j].index = j;
    }

    pub fn push(&mut self, entry: &Entry) {
        if self.has(&entry.id) {
            // avoid duplications
            return;
        }
        self.items.push_front(entry.to_owned());

        // insert key only if it does not already exist.
        self.lookup.insert(entry.id, entry.to_owned());
    }

    /// Helper wrapper around pop_front.
    pub fn pop(&mut self) -> Option<Entry> {
        self.items.pop_front()
    }

    /// Returns and removes the first element of the list.
    pub fn pop_back(&mut self) -> Option<Entry> {
        self.items.pop_back()
    }

    /// Returns and removes the latest element of the list.
    pub fn pop_front(&mut self) -> Option<Entry> {
        self.items.pop_front()
    }

    /// Attempts to retrieve an entry from the inner lookup map.
    pub fn get(&self, id: &ids::Id) -> Option<Entry> {
        if let Some(entry) = self.lookup.get(id) {
            return Some(entry.clone());
        }
        None
    }

    pub fn has(&self, id: &ids::Id) -> bool {
        if let Some(_) = self.get(id) {
            return true;
        }
        false
    }
}
