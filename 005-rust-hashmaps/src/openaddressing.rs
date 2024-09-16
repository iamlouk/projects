use crate::{InsertStatus, hash};

/*
 * Has a size of 3x64 bits, 3 bytes are wasted in all cases!
 * Use 3 bytes to store hash?
 * Put the tag of the enum/union in its own bitmap?
 */
#[derive(Debug, Clone, PartialEq)]
enum Entry {
    Used(u64, u64),
    Empty,
    Gone
}

pub struct Hashtable {
    nelms: usize,
    entries: Vec<Entry>
}

impl Hashtable {
    pub fn new(size: usize) -> Self {
        Self {
            nelms: 0,
            entries: vec![Entry::Empty; size.next_power_of_two()]
        }
    }

    fn next_index(&self, i: usize) -> usize {
        (i + 1) & (self.entries.len() - 1)
    }

    #[allow(dead_code)]
    fn rehash_inplace(&mut self) {
        let mut new_entries = vec![Entry::Empty; self.entries.len() * 2];
        let mask = new_entries.len() - 1;
        for e in &self.entries {
            if let Entry::Used(key, val) = *e {
                let pos0 = hash(key) & mask;
                let mut pos = pos0;
                loop {
                    match new_entries[pos] {
                        Entry::Empty => {
                            new_entries[pos] = Entry::Used(key, val);
                            break;
                        },
                        Entry::Used(k, _) if k == key => panic!("WTF?"),
                        Entry::Gone                   => panic!("WTF?"),
                        Entry::Used(_, _) => {
                            pos = (pos + 1) & mask;
                            if pos == pos0 {
                                panic!("WTF?");
                            }
                        }
                    }
                }
            }
        }

        self.entries = new_entries;
    }
}

impl crate::Hashtable for Hashtable {
    fn insert(&mut self, key: u64, val: u64) -> InsertStatus {
        let pos0 = hash(key) & (self.entries.len() - 1);
        let mut pos = pos0;
        loop {
            match self.entries[pos] {
                Entry::Used(k, _) if k == key => {
                    self.entries[pos] = Entry::Used(key, val);
                    return InsertStatus::Updated;
                },
                Entry::Empty | Entry::Gone => {
                    self.entries[pos] = Entry::Used(key, val);
                    self.nelms += 1;

                    if self.nelms * 2 > self.entries.len() * 3 {
                        self.rehash_inplace();
                    }

                    return InsertStatus::Inserted;
                },
                _ => {
                    pos = self.next_index(pos);
                    if pos == pos0 {
                        return InsertStatus::Full;
                    }
                }
            }
        }
    }

    fn lookup(&self, key: u64) -> Option<u64> {
        let pos0 = hash(key) & (self.entries.len() - 1);
        let mut pos = pos0;
        loop {
            match self.entries[pos] {
                Entry::Used(k, val) if k == key => {
                    return Some(val);
                },
                Entry::Empty => return None,
                _ => {
                    pos = self.next_index(pos);
                    if pos == pos0 {
                        return None;
                    }
                }
            }
        }
    }

    fn erase(&mut self, key: u64) -> Option<u64> {
        let pos0 = hash(key) & (self.entries.len() - 1);
        let mut pos = pos0;
        loop {
            match self.entries[pos] {
                Entry::Used(k, val) if k == key => {
                    self.entries[pos] = Entry::Gone;
                    return Some(val);
                },
                Entry::Empty => return None,
                _ => {
                    pos = self.next_index(pos);
                    if pos == pos0 {
                        return None;
                    }
                }
            }
        }
    }

    fn rehash(&self, dest: &mut Self) {
        for entry in &self.entries {
            if let Entry::Used(key, val) = *entry {
                assert_eq!(dest.insert(key, val), InsertStatus::Inserted);
            }
        }
    }
}

