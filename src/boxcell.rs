use std::cell::{Cell, RefCell};

/// An owned pointer that allows interior mutability
///
/// A [BoxCell] is currently the size of a four pointers.  Its size
/// could be decreased at the cost of a bit of code complexity if that
/// were deemed worthwhile.  By using a linked list of old values, we
/// could bring the common case down to 2 pointers.  Read access using
/// `BoxCell` is the same as for `Box`.
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
impl<T> Drop for BoxCell<T> {
    fn drop(&mut self) {
        // this frees the current value of the pointer.  It is acquire
        // because the contents of the pointer must be up to date so
        // the drop of T doesn't do anything crazy.
        unsafe { Box::from_raw(self.current.get()); }
    }
}

impl<T: Clone> BoxCell<T> {
    pub fn new(value: T) -> BoxCell<T> {
        BoxCell {
            current: Cell::new(Box::into_raw(Box::new(value))),
            old: RefCell::new(Vec::new()),
        }
    }
    /// Make a copy of the data and return a reference.
    ///
    /// When the guard is dropped, `self` will be updated.  There is
    /// no protection against two simultaneous writes.  The one that
    /// drops second will "win".
    pub fn update<'a>(&'a self) -> impl 'a + std::ops::DerefMut<Target=T> {
        BoxGuard {
            value: Box::into_raw(Box::new((*self).clone())),
            boxcell: self,
            guard: self.old.borrow_mut(),
        }
    }
    /// Free all old versions of the data.  Because this method
    /// requires a mutable reference, it is guaranteed that no
    /// references exist.
    pub fn clean(&mut self) {
        *self.old.borrow_mut() = Vec::new();
    }
}

impl<T> std::borrow::Borrow<T> for BoxCell<T> {
    fn borrow(&self) -> &T {
        &*self
    }
}

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

impl<T> std::ops::Deref for BoxCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            // I think I need to Acquire here because otherwise it may
            // be possible to update a pointer and then have the new
            // pointer value visible when the bytes that are pointed
            // to are still incorrect.
            &*self.current.get()
        }
    }
}

struct BoxGuard<'a,T: Clone> {
    value: *mut T,
    boxcell: &'a BoxCell<T>,
    guard: std::cell::RefMut<'a,Vec<Box<T>>>,
}
impl<'a,T: Clone> std::ops::Deref for BoxGuard<'a,T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}
impl<'a,T: Clone> std::ops::DerefMut for BoxGuard<'a,T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value }
    }
}
impl<'a,T: Clone> Drop for BoxGuard<'a,T> {
    fn drop(&mut self) {
        let oldvalue = self.boxcell.current.get();
        self.boxcell.current.set(self.value);
        self.guard.push(unsafe { Box::from_raw(oldvalue) });
    }
}
