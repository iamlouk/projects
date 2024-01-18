
mod taggedptr;
use taggedptr::TaggedPtr;

#[derive(Clone, Copy, PartialEq)]
enum Insert {
    Inserted,
    Replaced,
    Full
}

trait ArtNode<T: Sized> {
    fn get_entry(&self, key: u8) -> Option<&TaggedPtr<Node<T>, T>>;
    fn get_entry_mut(&mut self, key: u8) -> Option<&mut TaggedPtr<Node<T>, T>>;
    fn set_entry(&mut self, key: u8, entry: TaggedPtr<Node<T>, T>) -> Insert;
    fn del_entry(&mut self, key: u8) -> bool;
    fn grow(self) -> Node<T>;
}

type Node4<T> = ArtNodeLinSearch<T, 4>;
type Node16<T> = ArtNodeLinSearch<T, 16>;

enum Node<T: Sized> {
    Node4(Node4<T>),
    Node16(Node16<T>),
    Node256(ArtNode256<T>)
}

impl<T: Sized> Node<T> {
    pub fn as_trait(&self) -> &dyn ArtNode<T> {
        match self {
            Node::Node4(node) => node,
            Node::Node16(node) => node,
            Node::Node256(node) => node
        }
    }
}

pub struct Art<T: Sized> {
    root: Node<T>
}

impl<T: Sized> Art<T> {
    pub fn new() -> Self {
        Self { root: Node::Node4(Node4::new()) }
    }
}

struct ArtNodeLinSearch<T: Sized, const N: usize> {
    keys: [u8; N],
    entries: [TaggedPtr<Node<T>, T>; N]
}

impl<T: Sized, const N: usize> ArtNodeLinSearch<T, N> {
    fn new() -> Self {
        Self { keys: [0; N], entries: [(); N].map(|_| Default::default()) }
    }
}

impl<T: Sized, const N: usize> ArtNode<T> for ArtNodeLinSearch<T, N> {
    fn get_entry(&self, key: u8) -> Option<&TaggedPtr<Node<T>, T>> {
        for i in 0..N {
            if self.keys[i] == key && !self.entries[i].is_null() {
                return Some(&self.entries[i])
            }
        }
        None
    }

    fn get_entry_mut(&mut self, key: u8) -> Option<&mut TaggedPtr<Node<T>, T>> {
        for i in 0..N {
            if self.keys[i] == key && !self.entries[i].is_null() {
                return Some(&mut self.entries[i])
            }
        }
        None
    }

    fn set_entry(&mut self, key: u8, entry: TaggedPtr<Node<T>, T>) -> Insert {
        assert!(!entry.is_null() && key != 0x0);
        let mut i = 0;
        while i < N && self.keys[i] != 0x0 {
            if self.keys[i] == key {
                assert!(!self.entries[i].is_null());
                self.entries[i] = entry;
                return Insert::Replaced
            }
            i += 1;
        }

        if i < N {
            assert!(self.entries[i].is_null());
            self.keys[i] = key;
            self.entries[i] = entry;
            return Insert::Inserted
        }

        Insert::Full
    }

    fn del_entry(&mut self, key: u8) -> bool {
        assert!(key != 0x0);

        let mut i = 0;
        let mut removed = false;
        while i < N && self.keys[i] != 0x0 {
            if self.keys[i] == key {
                assert!(!self.entries[i].is_null());
                self.entries[i].reset();
                self.keys[i] = 0x0;
                removed = true;
                break
            }
            i += 1;
        }

        // TODO: untested!
        if i < N && removed {
            while i + 1 < N {
                self.keys[i] = self.keys[i + 1];
                self.entries[i] = self.entries[i + 1].take();
                i += 1;
            }
            self.keys[i] = 0x0;
            self.entries[i] = TaggedPtr::default();
        }
        return removed
    }

    fn grow(mut self) -> Node<T> {
        match N {
            4 => {
                let mut n = Node16::new();
                for i in 0..N {
                    if self.keys[i] != 0x0 {
                        assert!(n.set_entry(self.keys[i], self.entries[i].take()) == Insert::Inserted);
                    }
                }
                Node::Node16(n)
            },
            16 => {
                let mut n = ArtNode256::new();
                for i in 0..N {
                    if self.keys[i] != 0x0 {
                        assert!(n.set_entry(self.keys[i], self.entries[i].take()) == Insert::Inserted);
                    }
                }
                Node::Node256(n)
            },
            _ => unreachable!()
        }
    }
}

struct ArtNode256<T: Sized> {
    entries: [TaggedPtr<Node<T>, T>; 256]
}

impl<T: Sized> ArtNode256<T> {
    fn new() -> Self {
        Self { entries: [(); 256].map(|_| Default::default()) }
    }
}

impl<T: Sized> ArtNode<T> for ArtNode256<T> {
    fn get_entry(&self, key: u8) -> Option<&TaggedPtr<Node<T>, T>> {
        let entry = &self.entries[key as usize];
        return (!entry.is_null()).then_some(entry)
    }

    fn get_entry_mut(&mut self, key: u8) -> Option<&mut TaggedPtr<Node<T>, T>> {
        let entry = &mut self.entries[key as usize];
        return (!entry.is_null()).then_some(entry)
    }

    fn set_entry(&mut self, key: u8, entry: TaggedPtr<Node<T>, T>) -> Insert {
        assert!(!entry.is_null() && key != 0x0);
        let was_empty = self.entries[key as usize].is_null();
        self.entries[key as usize] = entry;
        if was_empty { Insert::Inserted } else { Insert::Replaced }
    }

    fn del_entry(&mut self, key: u8) -> bool {
        let entry = &mut self.entries[key as usize];
        let was_empty = entry.is_null();
        entry.reset();
        was_empty
    }

    fn grow(self) -> Node<T> {
        unreachable!("cannot grow a ART node with 256 slots")
    }
}

#[cfg(test)]
mod tests {
}
