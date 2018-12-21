//! This crate provides a set (eventually) of smart pointer types that
//! allow read access with no guards (and minimal to no overhead) and
//! no need to call [std::borrow::Borrow].  These smart pointers each
//! allow internal mutability (obtaining mutable references) by a
//! Read-Copy-Update approach, so you get a mutable reference to a
//! private copy of the data, which you can mutate at will.  When the
//! mutation is complete the pointer is atomically updated.  Old
//! references to the data may still exist, and will still be a valid
//! reference to the old data.
//!
//! Basically, these smart pointers allow internal mutability through
//! a slow and painful process, while keeping read-only access both
//! fast and easy (in particular, no need to call `ptr.borrow()`
//! everywhere).  Write access is guarded, but read access is not.
//!
//! The names of the types are based on the standard smart pointer
//! types.
//!
//! 1. `[BoxCell]` is an owned pointer similar to [std::box::Box].
//!    If you like, it is actually closer to `Box<RefCell<T>>`, but
//!    without the nuisance of having to call borrow when reading.

use std::cell::UnsafeCell;
use std::rc::Rc;

/// A reference counted pointer that allows interior mutability
///
/// An [RcCell] is currently the size of a five pointers and has an
/// additial layer of indirection.  Its size could be reduced at the
/// cost of a bit of code complexity if that were deemed worthwhile.
/// By using a linked list of old values, we could save a couple of
/// words.  Read access using `RcCell` has one additional indirection.

/// ```
/// let x = unguarded::RcCell::new(3);
/// let y: &usize = &(*x);
/// let z = x.clone();
/// *x.write() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// assert_eq!(*z, 7); // but the cloned pointer also points to the new value.
/// ```

#[derive(Clone)]
pub struct RcCell<T>(Rc<UnsafeCell<BoxCellInner<T>>>);

impl<T: Clone> RcCell<T> {
    pub fn new(value: T) -> RcCell<T> {
        RcCell( Rc::new(UnsafeCell::new(BoxCellInner {
            current: Box::new(value),
            old: Vec::new(),
        })))
    }
    /// Make a copy of the data and return a reference.
    ///
    /// When the guard is dropped, `self` will be updated.  There is
    /// no protection against two simultaneous writes.  The one that
    /// drops second will "win".
    pub fn write<'a>(&'a self) -> impl 'a + std::ops::DerefMut<Target=T> {
        unsafe {
            Guard {
                value: Box::new((*(*self)).clone()),
                inner: &mut *self.0.get(),
            }
        }
    }
}

impl<T> std::ops::Deref for RcCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            &(*self.0.get()).current
        }
    }
}

impl<T> std::borrow::Borrow<T> for RcCell<T> {
    fn borrow(&self) -> &T {
        &*self
    }
}

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
/// *x.write() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// ```
pub struct BoxCell<T>(UnsafeCell<BoxCellInner<T>>);
pub struct BoxCellInner<T> {
    current: Box<T>,
    old: Vec<Box<T>>,
}

impl<T: Clone> BoxCell<T> {
    pub fn new(value: T) -> BoxCell<T> {
        BoxCell( UnsafeCell::new(BoxCellInner {
            current: Box::new(value),
            old: Vec::new(),
        }))
    }
    /// Make a copy of the data and return a reference.
    ///
    /// When the guard is dropped, `self` will be updated.  There is
    /// no protection against two simultaneous writes.  The one that
    /// drops second will "win".
    pub fn write<'a>(&'a self) -> impl 'a + std::ops::DerefMut<Target=T> {
        unsafe {
            Guard {
                value: Box::new((*self).clone()),
                inner: &mut *self.0.get(),
            }
        }
    }
    /// Free all old versions of the data.  Because this method
    /// requires a mutable reference, it is guaranteed that no
    /// references exist.
    pub fn clean(&mut self) {
        unsafe {
            (*self.0.get()).old = Vec::new();
        }
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
        unsafe {
            let b = &mut (*self.0.get());
            b.old = Vec::new();
            b.current.borrow_mut()
        }
    }
}

impl<T> std::ops::Deref for BoxCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            &(*self.0.get()).current
        }
    }
}

struct Guard<'a,T: Clone> {
    value: Box<T>,
    inner: &'a mut BoxCellInner<T>,
}
impl<'a,T: Clone> std::ops::Deref for Guard<'a,T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.value
    }
}
impl<'a,T: Clone> std::ops::DerefMut for Guard<'a,T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.value
    }
}
impl<'a,T: Clone> Drop for Guard<'a,T> {
    fn drop(&mut self) {
        // FIXME I'd like to avoid the needless clone here.  Do I
        // really need to use an Option<Box<T>> just to avoid
        // allocating something to destroy?
        self.inner.old.push(std::mem::replace(&mut self.inner.current,
                                              self.value.clone()));
    }
}
