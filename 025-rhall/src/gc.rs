use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::cell::RefCell;

/*
 * Nothing of this is used in the actual compiler yet,
 * this is purely for experimenting and might replace
 * a lot of Rc<...> in the rest of the project.
 *
 *
 *
 */

#[derive(Clone, Copy, PartialEq)]
enum GCMark {
    Marked,
    Unmarked,
    Dead
}

pub struct GarbageCollector {
    // TODO: Replace this by a linked list for cheap insert/delete!
    // TODO: Put predecessor link in the (tuple)?
    tid: std::thread::ThreadId,
    stats_marked: usize,
    stats_sweeped: usize,
    allocations: std::collections::HashSet<std::ptr::NonNull<(GCMark, dyn Traceable)>>,
    toremove: Vec<std::ptr::NonNull<(GCMark, dyn Traceable)>>,
    roots: std::collections::HashSet<std::ptr::NonNull<(GCMark, dyn Traceable)>>
}

pub trait Traceable: std::any::Any {
    fn trace(&self, gc: &mut GarbageCollector);
}

pub struct GC<T> where T: Traceable {
    ptr: std::ptr::NonNull<(GCMark, dyn Traceable)>,
    phantom: std::marker::PhantomData<T>
}

impl<T: Traceable> Traceable for GC<T> {
    fn trace(&self, gc: &mut GarbageCollector) { gc.mark(self); }
}

impl<T: Traceable> Deref for GC<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { (&self.ptr.as_ref().1 as &dyn std::any::Any).downcast_ref_unchecked() }
    }
}

impl<T: Traceable> DerefMut for GC<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { (&mut self.ptr.as_mut().1 as &mut dyn std::any::Any).downcast_mut_unchecked() }
    }
}

impl GarbageCollector {
    pub unsafe fn mark_and_sweep(&mut self) {
        let num_allocs_start = self.allocations.len();
        self.stats_marked = 0;
        self.stats_sweeped = 0;
        for root in self.roots.iter().cloned() {
            (*root.as_ptr()).0 = GCMark::Marked;
            unsafe {
                // This is against Rust's rules and technically UB.
                // If trace() would, e.g., modify self.roots, we are fucked!
                let this = (self as *const Self) as *mut Self;
                (*root.as_ptr()).1.trace(&mut *this);
            }
        }

        self.sweep();
        eprintln!("GC: #allocations={}, #marked={}, #sweeped={}", num_allocs_start, self.stats_marked, self.stats_sweeped);
    }

    pub fn mark<T: Traceable>(&mut self, obj: &GC<T>) {
        unsafe {
            let obj = &mut *obj.ptr.as_ptr();
            assert!(obj.0 != GCMark::Dead);
            if obj.0 != GCMark::Marked {
                self.stats_marked += 1;
                obj.0 = GCMark::Marked;
                obj.1.trace(self);
            }
        }
    }

    pub unsafe fn sweep(&mut self) -> usize {
        self.toremove.clear();
        for a in self.allocations.iter() {
            assert!(a.as_ref().0 != GCMark::Dead);
            if a.as_ref().0 == GCMark::Marked {
                (*a.as_ptr()).0 = GCMark::Unmarked;
                continue;
            }

            drop(Box::from_raw(&mut (*a.as_ptr()).1 as *mut dyn Traceable));
            self.toremove.push(a.clone());
            self.stats_sweeped += 1;
        }

        for a in self.toremove.iter() {
            self.allocations.remove(a);
        }
        self.toremove.len()
    }

    pub fn wrap<T: Traceable>(&mut self, obj: T) -> GC<T> {
        unsafe {
            // Let's really stress the GC and mark&sweep for every allocation!
            self.mark_and_sweep();
        }
        let gc = GC {
            ptr: unsafe {
                let b: Box<(GCMark, dyn Traceable)> = Box::new((GCMark::Unmarked, obj));
                let p = std::ptr::NonNull::new_unchecked(Box::into_raw(b));
                self.allocations.insert(p);
                p
            },
            phantom: std::marker::PhantomData::<T>::default()
        };
        gc
    }
}

pub static mut GLOBAL_GC: *const GarbageCollector = std::ptr::null();

pub fn new<T: Traceable>(obj: T) -> GC<T> {
    let gc: &mut GarbageCollector = unsafe {
        // This is all of the things rust hates: not thread safe,
        // racy, ...
        if GLOBAL_GC.is_null() {
            GLOBAL_GC = Box::leak(Box::new(GarbageCollector {
                tid: std::thread::current().id(),
                stats_marked: 0,
                stats_sweeped: 0,
                allocations: std::collections::HashSet::new(),
                toremove: Vec::new(),
                roots: std::collections::HashSet::new()
            })) as *const _;
        }

        &mut *(GLOBAL_GC as *mut _)
    };

    gc.wrap(obj)
}

pub fn root<T: Traceable>(obj: &GC<T>) {
    let gc: &mut GarbageCollector = unsafe { &mut *(GLOBAL_GC as *mut _) };
    let inserted = gc.roots.insert(obj.ptr.clone());
    assert!(inserted);
}

pub fn unroot<T: Traceable>(obj: &GC<T>) {
    let gc: &mut GarbageCollector = unsafe { &mut *(GLOBAL_GC as *mut _) };
    let removed = gc.roots.remove(&obj.ptr);
    assert!(removed);
}

pub enum Type {
    Bool,
    Int,
    Str,
    Option(GC<Type>),
    Record(Vec<(Rc<str>, GC<Type>)>),
    Lambda(Vec<(Rc<str>, GC<Type>)>, GC<Type>)
}

impl Traceable for Type {
    fn trace(&self, gc: &mut GarbageCollector) {
        match self {
            Self::Bool | Self::Int | Self::Str => {},
            Self::Option(t) => t.trace(gc),
            Self::Record(fields) => {
                for (_, t) in fields {
                    t.trace(gc)
                }
            }
            Self::Lambda(args, ret) => {
                for (_, t) in args {
                    t.trace(gc)
                }
                ret.trace(gc)
            }
        }
    }
}

pub enum Value {
    Bool(bool),
    Int(i64),
    Str(Rc<str>),
    Option(Result<GC<Value>, GC<Type>>),
    Record(Vec<(Rc<str>, GC<Value>)>),
    Lambda(Vec<(Rc<str>, GC<Type>)>, Rc<RefCell<crate::ast::Node>>, GC<Scope>)
}

impl Traceable for Value {
    fn trace(&self, gc: &mut GarbageCollector) {
        match self {
            Self::Bool(_) | Self::Int(_) | Self::Str(_) => {},
            Self::Option(Ok(v)) => v.trace(gc),
            Self::Option(Err(t)) => t.trace(gc),
            Self::Record(fields) => {
                for (_, v) in fields {
                    v.trace(gc)
                }
            }
            Self::Lambda(args, _, scope) => {
                for (_, t) in args {
                    t.trace(gc)
                }
                scope.trace(gc)
            }
        }
    }
}

pub struct Scope {
    up: Option<GC<Scope>>,
    name: Rc<str>,
    value: GC<Value>
}

impl Traceable for Scope {
    fn trace(&self, gc: &mut GarbageCollector) {
        self.value.trace(gc);
        if let Some(up) = &self.up {
            up.trace(gc)
        }
    }
}
