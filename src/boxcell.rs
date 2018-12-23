use std::cell::{Cell, RefCell};
use crate::{RCU};


/// An owned pointer that allows interior mutability
///
/// The [BoxCell] is functionally roughly equivalent to `Box<RefCell<T>>`,
/// except that reads (of the old value) may happen while a write is
/// taking place..
///
/// A [BoxCell] is currently the size of a four pointers.  Its size
/// could be decreased at the cost of a bit of code complexity if that
/// were deemed worthwhile.  By using a linked list of old values, we
/// could bring the common case down to 2 pointers in the common case.
/// Read access using `BoxCell` is the same as for `Box`.
///
/// The main thing that simplifies and speeds up `[BoxCell]` is that
/// it cannot be either cloned or shared across threads.  Thus we
/// don't need any fancy locking or use of atomics.
///
/// ```
/// let x = unguarded::BoxCell::new(3);
/// let y: &usize = &(*x);
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// ```
pub struct BoxCell<T> {
    current: Cell<*mut T>,
    old: RefCell<Vec<Box<T>>>,
}

impl<T: Clone> BoxCell<T> {
    /// Allocate a new BoxCell.
    pub fn new(value: T) -> BoxCell<T> {
        BoxCell {
            current: Cell::new(Box::into_raw(Box::new(value))),
            old: RefCell::new(Vec::new()),
        }
    }
}

impl<'a,T: 'a> RCU<'a> for BoxCell<T> {
    type Target = T;
    type OldGuard = std::cell::RefMut<'a, Vec<Box<T>>>;
    fn get_raw(&self) -> *mut T {
        self.current.get()
    }
    fn set_raw(&self, new: *mut T) -> *mut T {
        let v = self.current.get();
        self.current.set(new);
        v
    }
    fn get_old_guard(&'a self) -> Self::OldGuard {
        self.old.borrow_mut()
    }
}
crate::impl_rcu!(BoxCell);

impl<T: Clone> std::borrow::BorrowMut<T> for BoxCell<T> {
    fn borrow_mut(&mut self) -> &mut T {
        // Since we are mut, we know there are no other borrows out
        // there, so we can throw away any old references.  We also
        // don't need to bother cloning the data, since (again) there
        // are no other references out there.  Yay.
        self.clean();
        // I think it's safe to use a relaxed load here because we
        // have exclusive access to this BoxCell, so there must be
        // some other synchronization done.
        unsafe { &mut *self.current.get() }
    }
}
