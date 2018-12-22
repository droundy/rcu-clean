use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A reference counted pointer that allows interior mutability
///
/// The [RcCell] is functionally roughly equivalent to `Rc<RefCell<T>>`,
/// except that reads (of the old value) may happen while a write is
/// taking place.
///
/// An [RcCell] is currently the size of a five pointers and has an
/// additial layer of indirection.  Its size could be reduced at the
/// cost of a bit of code complexity if that were deemed worthwhile.
/// By using a linked list of old values, we could save a couple of
/// words.  Read access using `RcCell` has one additional indirection.
/// Due to this additional indirection, `RcCell<T>` is probably slower
/// for read accesses than `Rc<RefCell<T>>`.  The main reason to use
/// it is for the convenience of not calling `borrow()` on every read.
///
/// ```
/// let x = unguarded::RcCell::new(3);
/// let y: &usize = &(*x);
/// let z = x.clone();
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// assert_eq!(*z, 7); // but the cloned pointer also points to the new value.
/// ```
pub struct RcCell<T> {
    inner: Rc<Inner<T>>,
    have_borrowed: Cell<bool>,
}
impl<T: Clone> Clone for RcCell<T> {
    fn clone(&self) -> Self {
        RcCell {
            inner: self.inner.clone(),
            have_borrowed: Cell::new(false),
        }
    }
}
pub struct Inner<T> {
    current: Cell<*mut T>,
    old: RefCell<Vec<Box<T>>>,
    borrow_count: Cell<usize>,
}
impl<T> Drop for Inner<T> {
    fn drop(&mut self) {
        // this frees the current value of the pointer.  It is acquire
        // because the contents of the pointer must be up to date so
        // the drop of T doesn't do anything crazy.
        unsafe { Box::from_raw(self.current.get()); }
    }
}


impl<T: Clone> RcCell<T> {
    pub fn new(value: T) -> RcCell<T> {
        RcCell {
            inner: Rc::new(Inner {
                current: Cell::new(Box::into_raw(Box::new(value))),
                old: RefCell::new(Vec::new()),
                borrow_count: Cell::new(0),
            }),
            have_borrowed: Cell::new(false),
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
            guard: self.inner.old.borrow_mut(),
        };
        g
    }
    /// Free all old versions of the data if possible.  Because this
    /// method requires a mutable reference, it is guaranteed that no
    /// references exist.
    pub fn clean(&mut self) {
        if self.have_borrowed.get() {
            self.inner.borrow_count.set(self.inner.borrow_count.get() - 1);
            if self.inner.borrow_count.get() == 0 {
                *self.inner.old.borrow_mut() = Vec::new();
            }
            self.have_borrowed.set(false);
        }
    }
}
impl<T> std::ops::Deref for RcCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let aleady_borrowed = self.have_borrowed.get();
        if !aleady_borrowed {
            self.inner.borrow_count.set(self.inner.borrow_count.get() + 1);
            self.have_borrowed.set(true); // indicate we have borrowed this once.
        }
        unsafe { &*self.inner.current.get() }
    }
}

impl<T> std::borrow::Borrow<T> for RcCell<T> {
    fn borrow(&self) -> &T {
        &*self
    }
}

struct Guard<'a,T: Clone> {
    value: *mut T,
    boxcell: &'a RcCell<T>,
    guard: std::cell::RefMut<'a,Vec<Box<T>>>,
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
        let oldvalue = self.boxcell.inner.current.get();
        self.boxcell.inner.current.set(self.value);
        self.guard.push(unsafe { Box::from_raw(oldvalue) });
    }
}
