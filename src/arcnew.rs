use std::cell::{Cell, UnsafeCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicPtr, Ordering};
use std::ptr::null_mut;

/// A reference counted pointer that allows interior mutability
///
/// The [ArcNew] is functionally roughly equivalent to `Arc<RefCell<T>>`,
/// except that reads (of the old value) may happen while a write is
/// taking place.
///
/// An [ArcNew] is currently the size of a five pointers and has an
/// additial layer of indirection.  Its size could be reduced at the
/// cost of a bit of code complexity if that were deemed worthwhile.
/// By using a linked list of old values, we could save a couple of
/// words.  Read access using `ArcNew` has one additional indirection.
/// Due to this additional indirection, `ArcNew<T>` is probably slower
/// for read accesses than `Arc<RefCell<T>>`.  The main reason to use
/// it is for the convenience of not calling `borrow()` on every read.
///
/// ```
/// let x = unguarded::ArcNew::new(3);
/// let y: &usize = &(*x);
/// let z = x.clone();
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// assert_eq!(*z, 7); // but the cloned pointer also points to the new value.
/// ```
pub struct ArcNew<T> {
    inner: Arc<Inner<T>>,
    have_borrowed: Cell<bool>,
}
impl<T: Clone> Clone for ArcNew<T> {
    fn clone(&self) -> Self {
        ArcNew {
            inner: self.inner.clone(),
            have_borrowed: Cell::new(false),
        }
    }
}
pub struct Inner<T> {
    borrow_count: AtomicUsize,
    am_writing: AtomicBool,
    list: List<T>
}
pub struct List<T> {
    value: UnsafeCell<T>,
    next: AtomicPtr<List<T>>,
}

impl<T> std::ops::Deref for ArcNew<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let aleady_borrowed = self.have_borrowed.get();
        if !aleady_borrowed {
            self.inner.borrow_count.fetch_add(1, Ordering::Relaxed);
            self.have_borrowed.set(true); // indicate we have borrowed this once.
        }
        let next = self.inner.list.next.load(Ordering::Acquire);
        if next == null_mut() {
            unsafe { &* self.inner.list.value.get() }
        } else {
            unsafe { &* (*next).value.get() }
        }
    }
}
impl<T> std::borrow::Borrow<T> for ArcNew<T> {
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
impl<'a,T: Clone> ArcNew<T> {
    pub fn new(x: T) -> Self {
        ArcNew {
            have_borrowed: Cell::new(false),
            inner: Arc::new(Inner {
                borrow_count: AtomicUsize::new(0),
                am_writing: AtomicBool::new(false),
                list: List {
                    value: UnsafeCell::new(x),
                    next: AtomicPtr::new(null_mut()),
                },
            }),
        }
    }
    pub fn update(&'a self) -> Guard<'a, T> {
        if self.inner.am_writing.swap(true, Ordering::Relaxed) {
            panic!("Cannont update an RcNew twice simultaneously.");
        }
        Guard {
            list: Some(List {
                value: UnsafeCell::new((*(*self)).clone()),
                next: AtomicPtr::new(self.inner.list.next.load(Ordering::Acquire)),
            }),
            rc_guts: &self.inner,
        }
    }
    pub fn clean(&mut self) {
        let aleady_borrowed = self.have_borrowed.get();
        if aleady_borrowed {
            self.inner.borrow_count.fetch_sub(1, Ordering::Relaxed);
            self.have_borrowed.set(false); // indicate we have no longer borrowed this.
        }
        let borrow_count = self.inner.borrow_count.load(Ordering::Relaxed);
        let next = self.inner.list.next.load(Ordering::Acquire);
        if borrow_count == 0 && next != null_mut() {
            unsafe {
                // make a copy of the old datum that we will need to free
                let buffer: UnsafeCell<Option<T>> = UnsafeCell::new(None);
                std::ptr::copy_nonoverlapping(self.inner.list.value.get(),
                                              buffer.get() as *mut T, 1);
                // now copy the "good" value to the main spot
                std::ptr::copy_nonoverlapping((*next).value.get(),
                                              self.inner.list.value.get(), 1);
                // Now we can set the pointer to null which activates
                // the copy we just made.
                let _to_be_freed = Box::from_raw(self.inner.list.next.swap(null_mut(), Ordering::Release));
                std::ptr::copy_nonoverlapping(buffer.get() as *mut T,
                                              (*next).value.get(), 1);
                let buffer_copy: UnsafeCell<Option<T>> = UnsafeCell::new(None);
                std::ptr::copy_nonoverlapping(buffer_copy.get(), buffer.get(), 1);
            }
        }
    }
}

pub struct Guard<'a,T: Clone> {
    list: Option<List<T>>,
    rc_guts: &'a Inner<T>,
}
impl<'a,T: Clone> std::ops::Deref for Guard<'a,T> {
    type Target = T;
    fn deref(&self) -> &T {
        if let Some(ref list) = self.list {
            unsafe { & *list.value.get() }
        } else {
            unreachable!()
        }
    }
}
impl<'a,T: Clone> std::ops::DerefMut for Guard<'a,T> {
    fn deref_mut(&mut self) -> &mut T {
        if let Some(ref list) = self.list {
            unsafe { &mut *list.value.get() }
        } else {
            unreachable!()
        }
    }
}
impl<'a,T: Clone> Drop for Guard<'a,T> {
    fn drop(&mut self) {
        let list = std::mem::replace(&mut self.list, None);
        self.rc_guts.list.next.store(Box::into_raw(Box::new(list.unwrap())),
                                     Ordering::Release);
        self.rc_guts.am_writing.store(false, Ordering::Relaxed);
    }
}
