use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr::null_mut;

/// A thread-safe owned pointer that allows interior mutability
///
/// An [BoxNew] is currently the size of a five pointers and has an
/// additial layer of indirection.  Its size could be reduced at the
/// cost of a bit of code complexity if that were deemed worthwhile.
/// By using a linked list of old values, we could save a couple of
/// words.  Read access using `BoxNew` has one additional indirection.

/// ```
/// let x = unguarded::BoxNew::new(3);
/// let y: &usize = &(*x);
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// ```
pub struct BoxNew<T> {
    inner: AtomicPtr<List<T>>,
}
pub struct List<T> {
    value: T,
    next: AtomicPtr<List<T>>,
}

impl<T> std::ops::Deref for BoxNew<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &(*self.inner.load(Ordering::Acquire)).value }
    }
}
impl<T> std::borrow::Borrow<T> for BoxNew<T> {
    fn borrow(&self) -> &T {
        &*self
    }
}
impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let next = self.next.load(Ordering::Acquire);
        if next != null_mut() {
            unsafe { Box::from_raw(next); }
        }
    }
}
impl<'a,T: Clone> BoxNew<T> {
    pub fn new(x: T) -> Self {
        BoxNew {
            inner: AtomicPtr::new(Box::into_raw(Box::new(List {
                value: x,
                next: AtomicPtr::new(null_mut()),
            }))),
        }
    }
    pub fn update(&'a self) -> Guard<'a, T> {
        Guard {
            list: AtomicPtr::new(Box::into_raw(Box::new(List {
                value: (*(*self)).clone(),
                next: unsafe { AtomicPtr::new((*self.inner.load(Ordering::Acquire))
                                              .next.load(Ordering::Acquire)) },
            }))),
            thebox: &self,
        }
    }
    pub fn clean(&mut self) {
        let inner = self.inner.load(Ordering::Acquire);
        let next = unsafe { (*inner).next.swap(null_mut(), Ordering::Acquire) };
        unsafe { Box::from_raw(next); }
    }
}

pub struct Guard<'a,T: Clone> {
    list: AtomicPtr<List<T>>,
    thebox: &'a BoxNew<T>,
}
impl<'a,T: Clone> std::ops::Deref for Guard<'a,T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &(*self.list.load(Ordering::Acquire)).value }
    }
}
impl<'a,T: Clone> std::ops::DerefMut for Guard<'a,T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.list.load(Ordering::Acquire)).value }
    }
}
impl<'a,T: Clone> Drop for Guard<'a,T> {
    fn drop(&mut self) {
        self.thebox.inner.store(self.list.load(Ordering::Acquire), Ordering::Release);
    }
}
