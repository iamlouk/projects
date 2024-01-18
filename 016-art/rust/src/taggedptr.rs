use std::marker::PhantomData;

pub struct TaggedPtr<L: Sized, R: Sized> {
    ptr: usize,
    _p1: std::marker::PhantomData<L>,
    _p2: std::marker::PhantomData<R>
}

impl<L: Sized, R: Sized> TaggedPtr<L, R> {
    pub fn new_null() -> Self {
        Self {
            ptr: 0,
            _p1: std::marker::PhantomData,
            _p2: std::marker::PhantomData
        }
    }

    pub fn is_null(&self) -> bool {
        self.ptr == 0
    }

    pub fn new_left(l: Box<L>) -> Self {
        let l_ptr = Box::into_raw(l) as usize;
        assert!((l_ptr & (1 << 63)) == 0);
        Self {
            ptr: l_ptr,
            _p1: std::marker::PhantomData,
            _p2: std::marker::PhantomData
        }
    }

    pub fn new_right(r: Box<R>) -> Self {
        let r_ptr: usize = Box::into_raw(r) as usize;
        assert!((r_ptr & (1 << 63)) == 0);
        Self {
            ptr: r_ptr | (1 << 63),
            _p1: std::marker::PhantomData,
            _p2: std::marker::PhantomData
        }
    }

    pub fn left(&self) -> Option<&L> {
        if self.ptr == 0 || self.ptr & (1 << 63) != 0 {
            return None;
        }
        return unsafe { Some(&*(self.ptr as *mut L)) }
    }

    pub fn left_mut(&mut self) -> Option<&mut L> {
        if self.ptr == 0 || self.ptr & (1 << 63) != 0 {
            return None;
        }
        return unsafe { Some(&mut *(self.ptr as *mut L)) }
    }

    pub fn right(&self) -> Option<&R> {
        if self.ptr == 0 || self.ptr & (1 << 63) == 0 {
            return None;
        }
        return unsafe { Some(&*((self.ptr & ((1 << 63) - 1)) as *mut R)) }
    }

    pub fn right_mut(&mut self) -> Option<&mut R> {
        if self.ptr == 0 || self.ptr & (1 << 63) == 0 {
            return None;
        }
        return unsafe { Some(&mut *((self.ptr & ((1 << 63) - 1)) as *mut R)) }
    }

    pub fn set_left(&mut self, l: Box<L>) {
        self.reset();
        let l_ptr = Box::into_raw(l) as usize;
        assert!((l_ptr & (1 << 63)) == 0);
        self.ptr = l_ptr;
    }

    pub fn set_right(&mut self, r: Box<R>) {
        self.reset();
        let r_ptr = Box::into_raw(r) as usize;
        assert!((r_ptr & (1 << 63)) == 0);
        self.ptr = r_ptr | (1 << 63);
    }

    pub fn reset(&mut self) {
        if self.ptr != 0 && self.ptr & (1 << 63) == 0 {
            drop(unsafe { Box::from_raw(self.ptr as *mut L) })
        } else if self.ptr & (1 << 63) != 0 {
            drop(unsafe { Box::from_raw((self.ptr & ((1 << 63) - 1)) as *mut R) })
        }
        self.ptr = 0;
    }

    pub fn take(&mut self) -> Self {
        let taken = Self { ptr: self.ptr, _p1: PhantomData, _p2: PhantomData };
        self.ptr = 0;
        taken
    }
}

impl<L: Sized, R: Sized> Drop for TaggedPtr<L, R> {
    fn drop(&mut self) { self.reset(); }
}

impl<L: Sized, R: Sized> std::default::Default for TaggedPtr<L, R> {
    fn default() -> Self { Self::new_null() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tagged_ptr() {
        assert!(std::mem::size_of::<TaggedPtr<f64, u128>>() == 8);

        let val1 = Box::new(3.14);
        let val2 = Box::new(42);

        let mut tptr1: TaggedPtr<f64, u64> = TaggedPtr::new_left(val1);
        assert!(*tptr1.left().unwrap() == 3.14);
        assert!(tptr1.right().is_none());

        let mut tptr2: TaggedPtr<f64, u64> = TaggedPtr::new_right(val2);
        assert!(*tptr2.right().unwrap() == 42);
        assert!(tptr2.left().is_none());

        tptr1.set_right(Box::new(123));
        tptr2.set_left(Box::new(1.41));
        assert!(*tptr1.right().unwrap() == 123);
        assert!(*tptr2.left().unwrap() == 1.41);

        tptr1.reset();
        assert!(tptr1.is_null());
    }
}
