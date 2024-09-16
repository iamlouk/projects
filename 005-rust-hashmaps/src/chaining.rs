use crate::{InsertStatus, hash};

#[derive(Debug, Clone, PartialEq)]
struct Entry {
    key: u64,
    val: u64,
    next: Option<Box<Entry>>
}

pub struct Hashtable {
    nelms: usize,
    entries: Vec<Option<Box<Entry>>>
}

impl Hashtable {
    pub fn new(size: usize) -> Self {
        Self {
            nelms: 0,
            entries: vec![None; size.next_power_of_two()]
        }
    }

    // like rehash, but inside of the same hashtable and re-uses the entry boxes to avoid
    // extra allocations by `Box::new(...)`.
    #[allow(dead_code)]
    fn rehash_inplace(&mut self) {
        let mut new_entries: Vec<Option<Box<Entry>>> = vec![None; self.entries.len() * 2];
        for first in &mut self.entries {
            let mut eopt = first.take();
            while let Some(mut e) = eopt {
                let next = e.next.take();
                Self::insert_entry(&mut new_entries, e);
                eopt = next;
            }
        }

        self.entries = new_entries;
    }

    // Only inserts new keys, does not update existing entries!
    #[allow(dead_code)]
    fn insert_entry(entries: &mut Vec<Option<Box<Entry>>>, mut e: Box<Entry>) {
        let pos = hash(e.key) & (entries.len() - 1);
        let eopt = &mut entries[pos];
        e.next = eopt.take();
        *eopt = Some(e);
    }
}

impl crate::Hashtable for Hashtable {
    fn insert(&mut self, key: u64, val: u64) -> InsertStatus {
        let pos = hash(key) & (self.entries.len() - 1);
        let mut eopt = &mut self.entries[pos];
        while let Some(e) = eopt {
            if e.key == key {
                e.val = val;
                return InsertStatus::Updated;
            }
            eopt = &mut e.next;
        }

        self.nelms += 1;

        // if self.nelms * 2 > self.entries.len() * 3 {
        //  self.rehash_inplace();
        // }

        let ne = Box::new(Entry{
            key,
            val,
            next: self.entries[pos].take()
        });
        self.entries[pos] = Some(ne);
        InsertStatus::Inserted
    }

    fn lookup(&self, key: u64) -> Option<u64> {
        let pos = hash(key) & (self.entries.len() - 1);
        let mut eopt = &self.entries[pos];
        while let Some(e) = eopt {
            if e.key == key {
                return Some(e.val)
            }
            eopt = &e.next;
        }

        None
    }

    fn erase(&mut self, key: u64) -> Option<u64> {
        let pos = hash(key) & (self.entries.len() - 1);
        let mut eopt = &mut self.entries[pos];
        loop {
            match eopt {
                Some(e) if e.key == key => {
                    self.nelms -= 1;
                    let val = e.val;
                    *eopt = e.next.take();
                    return Some(val);
                },
                Some(e) => eopt = &mut e.next,
                None => return None
            }
        }
    }

    fn rehash(&self, dest: &mut Self) {
        for first in &self.entries {
            let mut eopt = first;
            while let Some(e) = eopt {
                assert_eq!(dest.insert(e.key, e.val), InsertStatus::Inserted);
                eopt = &e.next;
            }
        }
    }
}

