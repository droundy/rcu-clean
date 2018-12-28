use std::cell::{Cell, UnsafeCell};
use std::rc::Rc;
use std::ptr::null_mut;

/// A reference counted pointer that allows interior mutability
///
/// The [RcRcu] is functionally roughly equivalent to
/// `Rc<RefCell<T>>`, except that reads (of the old value) may happen
/// while a write is taking place.  Reads are actually slightly slower
/// than an `Rc<RefCell<T>>`, so mostly you gaining ergonomics by
/// using this.
///
/// ```
/// let x = unguarded::RcRcu::new(3);
/// let y: &usize = &(*x);
/// let z = x.clone();
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// assert_eq!(*z, 7); // but the cloned pointer also points to the new value.
/// ```
pub struct RcRcu<T> {
    inner: Rc<Inner<T>>,
    have_borrowed: Cell<bool>,
}
impl<T: Clone> Clone for RcRcu<T> {
    fn clone(&self) -> Self {
        RcRcu {
            inner: self.inner.clone(),
            have_borrowed: Cell::new(false),
        }
    }
}
pub struct Inner<T> {
    borrow_count: Cell<usize>,
    am_writing: Cell<bool>,
    list: List<T>
}
pub struct List<T> {
    value: UnsafeCell<T>,
    next: Cell<*mut List<T>>,
}

impl<T> std::ops::Deref for RcRcu<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let aleady_borrowed = self.have_borrowed.get();
        if !aleady_borrowed {
            self.inner.borrow_count.set(self.inner.borrow_count.get() + 1);
            self.have_borrowed.set(true); // indicate we have borrowed this once.
        }
        if self.inner.list.next.get() == null_mut() {
            unsafe { &*self.inner.list.value.get() }
        } else {
            unsafe { &* (*self.inner.list.next.get()).value.get() }
        }
    }
}
impl<T> std::borrow::Borrow<T> for RcRcu<T> {
    fn borrow(&self) -> &T {
        &*self
    }
}
impl<T> Drop for List<T> {
    fn drop(&mut self) {
        if self.next.get() != null_mut() {
            unsafe { Box::from_raw(self.next.get()); }
        }
    }
}
impl<'a,T: Clone> RcRcu<T> {
    pub fn new(x: T) -> Self {
        RcRcu {
            have_borrowed: Cell::new(false),
            inner: Rc::new(Inner {
                borrow_count: Cell::new(0),
                am_writing: Cell::new(false),
                list: List {
                    value: UnsafeCell::new(x),
                    next: Cell::new(null_mut()),
                },
            }),
        }
    }
    pub fn update(&'a self) -> Guard<'a, T> {
        if self.inner.am_writing.get() {
            panic!("Cannont update an RcRcu twice simultaneously.");
        }
        self.inner.am_writing.set(true);
        Guard {
            list: Some(List {
                value: UnsafeCell::new((*(*self)).clone()),
                next: self.inner.list.next.clone(),
            }),
            rc_guts: &self.inner,
        }
    }
    pub fn clean(&mut self) {
        let aleady_borrowed = self.have_borrowed.get();
        if aleady_borrowed {
            self.inner.borrow_count.set(self.inner.borrow_count.get() - 1);
            self.have_borrowed.set(false); // indicate we have no longer borrowed this.
        }
        if self.inner.borrow_count.get() == 0 &&
            self.inner.list.next.get() != null_mut()
        {
            unsafe {
                std::mem::swap(&mut *self.inner.list.value.get(),
                               &mut *(*self.inner.list.next.get()).value.get());
                Box::from_raw(self.inner.list.next.replace(null_mut()));
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
        self.rc_guts.list.next.set(Box::into_raw(Box::new(list.unwrap())));
        self.rc_guts.am_writing.set(false);
    }
}
