#![allow(incomplete_features)]
#![feature(test)]
#![feature(generic_const_exprs)]

pub mod chaining;
pub mod openaddressing;
pub mod better_chaining;

extern crate rand;
extern crate test;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InsertStatus {
    Updated,
    Inserted,
    Full
}

pub trait Hashtable {
    fn insert(&mut self, key: u64, val: u64) -> InsertStatus;
    fn lookup(&self, key: u64) -> Option<u64>;
    fn erase(&mut self, key: u64) -> Option<u64>;
    fn rehash(&self, dest: &mut Self);
}

fn hash(mut k: u64) -> usize {
    let m = 0xc6a4a7935bd1e995u64;
    let r = 47u64;
    let mut h = 0x8445d61a4e774912 ^ (m.wrapping_mul(8));
    k = k.wrapping_mul(m);
    k ^= k >> r;
    k = k.wrapping_mul(m);
    h ^= k;
    h = h.wrapping_mul(m);
    h ^= h >> r;
    h = h.wrapping_mul(m);
    h ^= h >> r;
    h as usize
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use test::Bencher;
    use super::*;

    fn test_hashtable<T>(new_hastable: fn(usize) -> T) where T: Hashtable {
        let sizes: Vec<u64> = vec![10, 99, 837, 48329, 384933];
        for size in sizes {
            let mut hashtable = new_hastable(size as usize);

            for i in 0..size {
                let status = hashtable.insert(i, 42);
                assert_eq!(status, InsertStatus::Inserted);
            }

            for i in 0..size {
                let status = hashtable.insert(i, i);
                assert_eq!(status, InsertStatus::Updated);
            }

            for i in 0..size {
                let val = hashtable.lookup(i);
                assert_eq!(val, Some(i));
            }

            for i in (0..(size/2)).step_by(3) {
                assert!(hashtable.erase(i).is_some());
            }

            for i in (0..(size/2)).step_by(3) {
                assert!(hashtable.erase(i).is_none());
            }

            for i in 0..(size/2) {
                let res = hashtable.lookup(i);
                if i % 3 == 0 {
                    assert_eq!(res, None);
                } else {
                    assert_eq!(res, Some(i));
                }
            }

            let size2 = size * 2;
            let mut ht2 = new_hastable(size2 as usize);
            hashtable.rehash(&mut ht2);


            for i in 0..(size/2) {
                let res = ht2.lookup(i);
                if i % 3 == 0 {
                    assert_eq!(res, None);
                } else {
                    assert_eq!(res, Some(i));
                }
            }
        }
    }

    #[test]
    fn test_chaining() {
        test_hashtable(|size: usize| chaining::Hashtable::new(size));
    }

    #[test]
    fn test_better_chaining() {
        test_hashtable(|size: usize| better_chaining::Hashtable::new(size));
    }

    #[test]
    fn test_open_addressing() {
        test_hashtable(|size: usize| openaddressing::Hashtable::new(size));
    }


    fn bench_hashtable_lookups<T>(b: &mut Bencher, fill_factor: f64, new_hastable: fn(usize) -> T) where T: Hashtable {
        let table_size = 5_000_000u64;
        let inserts = ((table_size as f64) * fill_factor) as u64;
        let mut ht = new_hastable(table_size as usize);
        for i in 0..inserts {
            let key = rand::random::<u64>();
            ht.insert(key, i);
        }

        b.iter(|| {
            let key = rand::random::<u64>();
            ht.lookup(key)
        });
    }

    #[bench]
    fn bench_chaining_lookups_f99(b: &mut Bencher) {
        bench_hashtable_lookups(b, 0.99, |size: usize| chaining::Hashtable::new(size));
    }

    #[bench]
    fn bench_open_addressing_lookups_f99(b: &mut Bencher) {
        bench_hashtable_lookups(b, 0.99, |size: usize| openaddressing::Hashtable::new(size));
    }

    #[bench]
    fn bench_chaining_lookups_f50(b: &mut Bencher) {
        bench_hashtable_lookups(b, 0.5, |size: usize| chaining::Hashtable::new(size));
    }

    #[bench]
    fn bench_open_addressing_lookups_f50(b: &mut Bencher) {
        bench_hashtable_lookups(b, 0.5, |size: usize| openaddressing::Hashtable::new(size));
    }
}


