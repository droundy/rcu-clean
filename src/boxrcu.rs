use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr::null_mut;

/// An owned pointer that allows interior mutability
///
/// An [BoxRcu] is currently the size of two pointers (plus the
/// allocated data).  So one pointer of overhead versus a plain old
/// `Box`.  You will probably want to to occasionally call `[clean]`
/// to free up copies made when you call `update`.  Or you could just
/// leak memory, that's cool too.
///
/// Our benchmark oddly shows [BoxRcu] reads as being faster than
/// reads using [Box].  I don't understand this, or particularly
/// believe it.

/// ```
/// let x = rcu_clean::BoxRcu::new(3);
/// let y: &usize = &(*x);
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// ```
pub struct BoxRcu<T> {
    inner: AtomicPtr<List<T>>,
}
pub struct List<T> {
    value: T,
    next: AtomicPtr<List<T>>,
}

impl<T> std::ops::Deref for BoxRcu<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &(*self.inner.load(Ordering::Acquire)).value }
    }
}
impl<T> std::borrow::Borrow<T> for BoxRcu<T> {
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
impl<'a,T: Clone> BoxRcu<T> {
    pub fn new(x: T) -> Self {
        BoxRcu {
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
    thebox: &'a BoxRcu<T>,
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
