use std::cell::Cell;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, Mutex};
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
}
impl<T: Clone> Clone for ArcCell<T> {
    fn clone(&self) -> Self {
        ArcCell {
            inner: self.inner.clone(),
        }
    }
}
pub struct Inner<T> {
    current: AtomicPtr<T>,
    old: Mutex<Vec<Box<T>>>,
    borrow_count: AtomicUsize,
}
impl<T> Drop for Inner<T> {
    fn drop(&mut self) {
        // this frees the current value of the pointer.  It is acquire
        // because the contents of the pointer must be up to date so
        // the drop of T doesn't do anything crazy.
        unsafe { Box::from_raw(self.current.load(Ordering::Acquire)); }
    }
}


impl<T: Clone> ArcCell<T> {
    pub fn new(value: T) -> ArcCell<T> {
        ArcCell {
            inner: Arc::new(Inner {
                current: AtomicPtr::new(Box::into_raw(Box::new(value))),
                old: Mutex::new(Vec::new()),
                borrow_count: AtomicUsize::new(0),
            }),
        }
    }
    /// Make a copy of the data and return a reference.
    ///
    /// When the guard is dropped, `self` will be updated.  There is
    /// no protection against two simultaneous updates.  The one that
    /// drops second will "win".
    pub fn update<'a>(&'a self) -> impl 'a + std::ops::DerefMut<Target=T> {
        let g: Guard<'a,T> = Guard {
            value: Box::into_raw(Box::new((*(*self)).clone())),
            boxcell: self,
            guard: self.inner.old.lock().unwrap(),
        };
        g
    }
    /// Free all old versions of the data if possible.  Because this
    /// method requires a mutable reference, it is guaranteed that no
    /// references exist to *this* ArcCell, but we don't know about
    /// other ArcCells.  We could track borrows, but instead we simply restrict ourselves t
    pub fn clean(&mut self) {
        if self.inner.strong_count() == 1 {
            // We
        }
        if self.have_borrowed.get() {
            self.inner.borrow_count.set(self.inner.borrow_count.get() - 1);
            if self.inner.borrow_count.get() == 0 {
                *self.inner.old.borrow_mut() = Vec::new();
            }
            self.have_borrowed.set(false);
        }
    }
}
impl<T> std::ops::Deref for ArcCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let aleady_borrowed = self.have_borrowed.get();
        if !aleady_borrowed {
            self.inner.borrow_count.set(self.inner.borrow_count.get() + 1);
            self.have_borrowed.set(true); // indicate we have borrowed this once.
        }
        unsafe {
            // I think I need to Acquire here because otherwise it may
            // be possible to update a pointer and then have the new
            // pointer value visible when the bytes that are pointed
            // to are still incorrect.
            &*self.inner.current.load(Ordering::Acquire)
        }
    }
}

impl<T> std::borrow::Borrow<T> for ArcCell<T> {
    fn borrow(&self) -> &T {
        &*self
    }
}

struct Guard<'a,T: Clone> {
    value: *mut T,
    boxcell: &'a ArcCell<T>,
    guard: std::sync::MutexGuard<'a,Vec<Box<T>>>,
}
impl<'a,T: Clone> std::ops::Deref for Guard<'a,T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}
impl<'a,T: Clone> std::ops::DerefMut for Guard<'a,T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value }
    }
}
impl<'a,T: Clone> Drop for Guard<'a,T> {
    fn drop(&mut self) {
        self.value = self.boxcell.inner.current.swap(self.value, Ordering::Release);
        self.guard.push(unsafe { Box::from_raw(self.value) });
    }
}
