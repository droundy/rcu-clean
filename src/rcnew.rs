use std::cell::{Cell, RefCell};
use std::rc::Rc;
use crate::RCU;

/// A reference counted pointer that allows interior mutability
///
/// The [RcNew] is functionally roughly equivalent to `Rc<RefCell<T>>`,
/// except that reads (of the old value) may happen while a write is
/// taking place.
///
/// An [RcNew] is currently the size of a five pointers and has an
/// additial layer of indirection.  Its size could be reduced at the
/// cost of a bit of code complexity if that were deemed worthwhile.
/// By using a linked list of old values, we could save a couple of
/// words.  Read access using `RcNew` has one additional indirection.
/// Due to this additional indirection, `RcNew<T>` is probably slower
/// for read accesses than `Rc<RefCell<T>>`.  The main reason to use
/// it is for the convenience of not calling `borrow()` on every read.
///
/// ```
/// let x = unguarded::RcNew::new(3);
/// let y: &usize = &(*x);
/// let z = x.clone();
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// assert_eq!(*z, 7); // but the cloned pointer also points to the new value.
/// ```
pub struct RcNew<T> {
    inner: Rc<Inner<T>>,
    have_borrowed: Cell<bool>,
}
impl<T: Clone> Clone for RcNew<T> {
    fn clone(&self) -> Self {
        RcNew {
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

impl<'a,T: 'a> RCU<'a> for RcNew<T> {
    type Target = T;
    type OldGuard = std::cell::RefMut<'a, Vec<Box<T>>>;
    fn get_raw(&self) -> *mut T {
        let aleady_borrowed = self.have_borrowed.get();
        if !aleady_borrowed {
            self.inner.borrow_count.set(self.inner.borrow_count.get() + 1);
            self.have_borrowed.set(true); // indicate we have borrowed this once.
        }
        self.inner.current.get()
    }
    fn set_raw(&self, new: *mut T) -> *mut T {
        let v = self.inner.current.get();
        self.inner.current.set(new);
        v
    }
    fn get_old_guard(&'a self) -> Self::OldGuard {
        self.inner.old.borrow_mut()
    }
    fn release(&mut self) -> bool {
        if !self.have_borrowed.get() {
            return false;
        }
        self.have_borrowed.set(false);
        self.inner.borrow_count.set(self.inner.borrow_count.get() - 1);
        self.inner.borrow_count.get() == 0
    }
}
crate::impl_rcu!(RcNew);

impl<T: Clone> RcNew<T> {
    pub fn new(value: T) -> RcNew<T> {
        RcNew {
            inner: Rc::new(Inner {
                current: Cell::new(Box::into_raw(Box::new(value))),
                old: RefCell::new(Vec::new()),
                borrow_count: Cell::new(0),
            }),
            have_borrowed: Cell::new(false),
        }
    }
    /// Free all old versions of the data if possible.  Because this
    /// method requires a mutable reference, it is guaranteed that no
    /// references exist.
    pub fn clean(&mut self) {
        if self.have_borrowed.get() {
            self.inner.borrow_count.set(self.inner.borrow_count.get() - 1);
            if self.inner.borrow_count.get() == 0 {
                *self.get_old_guard() = Vec::new();
            }
            self.have_borrowed.set(false);
        }
    }
}
