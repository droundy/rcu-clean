use std::cell::Cell;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use crate::{RCU};
/// A thread-safe reference counted pointer that allows interior mutability
///
/// An [ArcCell] is currently the size of a five pointers and has an
/// additial layer of indirection.  Its size could be reduced at the
/// cost of a bit of code complexity if that were deemed worthwhile.
/// By using a linked list of old values, we could save a couple of
/// words.  Read access using `ArcCell` has one additional indirection.

/// ```
/// let x = unguarded::ArcCell::new(3);
/// let y: &usize = &(*x);
/// let z = x.clone();
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// assert_eq!(*z, 7); // but the cloned pointer also points to the new value.
/// ```
pub struct ArcCell<T> {
    inner: Arc<Inner<T>>,
    have_borrowed: Cell<bool>,
}
impl<T: Clone> Clone for ArcCell<T> {
    fn clone(&self) -> Self {
        ArcCell {
            inner: self.inner.clone(),
            have_borrowed: Cell::new(false),
        }
    }
}
pub struct Inner<T> {
    current: AtomicPtr<T>,
    old: Mutex<Vec<Box<T>>>,
    borrow_count: AtomicUsize,
}

impl<'a,T: 'a> RCU<'a> for ArcCell<T> {
    type Target = T;
    type OldGuard = std::sync::MutexGuard<'a, Vec<Box<T>>>;
    fn get_raw(&self) -> *mut T {
        if !self.have_borrowed.get() {
            self.inner.borrow_count.fetch_add(1, Ordering::Relaxed);
            self.have_borrowed.set(true);
        }
        self.inner.current.load(Ordering::Acquire)
    }
    fn set_raw(&self, new: *mut T) -> *mut T {
        self.inner.current.swap(new, Ordering::Release)
    }
    fn get_old_guard(&'a self) -> Self::OldGuard {
        self.inner.old.lock().unwrap()
    }
    fn release(&mut self) -> bool {
        if !self.have_borrowed.get() {
            return false;
        }
        self.have_borrowed.set(false);
        self.inner.borrow_count.fetch_sub(1, Ordering::Relaxed) == 1
    }
}
crate::impl_rcu!(ArcCell);

impl<T: Clone> ArcCell<T> {
    pub fn new(value: T) -> ArcCell<T> {
        ArcCell {
            have_borrowed: Cell::new(false),
            inner: Arc::new(Inner {
                current: AtomicPtr::new(Box::into_raw(Box::new(value))),
                old: Mutex::new(Vec::new()),
                borrow_count: AtomicUsize::new(0),
            }),
        }
    }
}
