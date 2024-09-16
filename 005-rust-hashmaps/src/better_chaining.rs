use crate::{InsertStatus, hash};

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    HasNext(i32), // Index into `chain`
    NoNext,
    Free,
}

#[derive(Debug, Clone, PartialEq)]
struct Entry {
    key: u64,
    val: u64,
    state: State,
}

pub struct Hashtable {
    entries: Vec<Entry>, // first entry directly embedded into the vector.
    chain: Vec<Entry>, // allocation space for elements that go in a chain.
}

impl Hashtable {
    pub fn new(size: usize) -> Self {
        println!("sizeof(better_chaining::Entry) = {:?}", std::mem::size_of::<Entry>());

        Self {
            entries: vec![Entry{ key: 0, val: 0, state: State::Free }; size.next_power_of_two()],
            chain:   Vec::with_capacity(size.next_power_of_two() / 3 + 1)
        }
    }

    #[allow(dead_code)]
    pub fn rehash_inplace(&mut self) {
        let mut tmp = Self::new(self.entries.len() * 2);

        crate::Hashtable::rehash(self, &mut tmp);

        self.entries = tmp.entries;
        self.chain = tmp.chain;
    }
}

impl crate::Hashtable for Hashtable {
    fn insert(&mut self, key: u64, val: u64) -> InsertStatus {
        let pos = hash(key) & (self.entries.len() - 1);
        if self.entries[pos].state != State::Free {
            let e0 = &mut self.entries[pos];
            // Update entry not in chain list:
            if e0.key == key {
                e0.val = val;
                return InsertStatus::Updated;
            }

            // Search the chain list:
            let mut prev_idx = -1;
            let mut next_idx = e0.state;
            while let State::HasNext(idx) = next_idx {
                let e = &mut self.chain[idx as usize];
                if e.key == key {
                    e.val = val;
                    return InsertStatus::Updated;
                }

                prev_idx = idx;
                next_idx = e.state;
            }

            // Insert a new entry in the chain:
            let new_idx = self.chain.len() as i32;
            self.chain.push(Entry{
                key,
                val,
                state: State::NoNext
            });
            if prev_idx == -1 {
                e0.state = State::HasNext(new_idx);
            } else {
                self.chain[prev_idx as usize].state = State::HasNext(new_idx);
            }
        } else {
            // Insert new entry in the "root" table:
            self.entries[pos] = Entry {
                key,
                val,
                state: State::NoNext
            };
        }

        // Chain is getting to large, rehash:
        if self.chain.len() * 3 >= self.entries.len() {
            self.rehash_inplace();
        }

        InsertStatus::Inserted
    }

    fn lookup(&self, key: u64) -> Option<u64> {
        let pos = hash(key) & (self.entries.len() - 1);
        let e0 = &self.entries[pos];
        if e0.state != State::Free {
            let mut e = e0;
            loop {
                if e.key == key {
                    return Some(e.val);
                }

                match e.state {
                    State::HasNext(idx) => e = &self.chain[idx as usize],
                    _ => break
                }
            }
        }

        None
    }

    fn erase(&mut self, key: u64) -> Option<u64> {
        let pos = hash(key) & (self.entries.len() - 1);
        let e0 = &mut self.entries[pos];
        if e0.state != State::Free {
            if e0.key == key {
                let val = e0.val;
                if let State::HasNext(idx) = e0.state {
                    self.entries[pos] = self.chain[idx as usize].clone();
                    self.chain[idx as usize].state = State::Free;
                } else {
                    self.entries[pos].state = State::Free;
                }

                return Some(val);
            }

            let mut prev_idx = -1;
            let mut next_idx = e0.state;
            while let State::HasNext(idx) = next_idx {
                let e = &mut self.chain[idx as usize];
                if e.key == key {
                    let val = e.val;
                    let next = e.state;
                    e.state = State::Free;
                    if prev_idx == -1 {
                        e0.state = next;
                    } else {
                        self.chain[prev_idx as usize].state = next;
                    }
                    return Some(val);
                }

                prev_idx = idx;
                next_idx = e.state;
            }
        }

        None
    }

    fn rehash(&self, dest: &mut Self) {
        for e in &self.entries {
            if e.state != State::Free {
                dest.insert(e.key, e.val);
            }
        }

        for e in &self.chain {
            if e.state != State::Free {
                dest.insert(e.key, e.val);
            }
        }
    }
}

