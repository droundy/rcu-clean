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
//!
//! 2. `[RcCell]` is a reference counted pointer similar to [std::rc::Rc].
//!    If you like, it is actually closer to `Rc<RefCell<T>>`, but
//!    without the nuisance of having to call borrow when reading.
//!
//! 3. `ArcCell` is planned to be a thread-safe reference counted
//!    pointer similar to [std::sync::Arc].  It is actually
//!    closer to `Arc<RwLock<T>>`, but without the nuisance of having to
//!    call `read` before reading.
//!
//! ### Cleaning
//!
//! Due to this crate's read-copy-update semantics, old copies of your
//! data are kept until we are confident that there are no longer any
//! references to them.  Because we do not have any guards on the read
//! references, this must be done manually.  This is the cost we pay
//! for extra convenience (and speed in the case of `BoxCell`) on the
//! read operations.  You have two options for how to handle this.
//!
//! One option is to simply store those extra copies until then entire
//! smart pointer itself is freed.  That is what happens if you do
//! nothing, and for small data that is only mutated once, it's a
//! great option.
//!
//! The other option is to call `clean()` when convenient.  `clean`
//! takes a `&mut self`, so when it is called, the compiler will prove
//! to us that there are no other references out there via *this*
//! smart pointer.  For `BoxCell` that is sufficient to prove that we
//! can free the data.  In the case of the reference counted data
//! pointers, we keep track of a count of how many copies have been
//! dereferenced since the last time `clean` was called.  We could do
//! better with "epoch" counting, but in most cases I don't think that
//! will be needed.

use std::cell::{UnsafeCell, Cell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool,Ordering};

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
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// assert_eq!(*z, 7); // but the cloned pointer also points to the new value.
/// ```

pub struct RcCell<T> {
    inner: Rc<UnsafeCell<BoxCellInner<T>>>,
    have_borrowed: AtomicBool,
}

impl<T: Clone> Clone for RcCell<T> {
    fn clone(&self) -> Self {
        RcCell {
            inner: self.inner.clone(),
            have_borrowed: AtomicBool::new(false),
        }
    }
}

impl<T: Clone> RcCell<T> {
    pub fn new(value: T) -> RcCell<T> {
        RcCell {
            inner: Rc::new(UnsafeCell::new(BoxCellInner {
                current: Box::new(value),
                old: Vec::new(),
                borrow_count: 0,
            })),
            have_borrowed: AtomicBool::new(false),
        }
    }
    /// Make a copy of the data and return a reference.
    ///
    /// When the guard is dropped, `self` will be updated.  There is
    /// no protection against two simultaneous updates.  The one that
    /// drops second will "win".
    pub fn update<'a>(&'a self) -> impl 'a + std::ops::DerefMut<Target=T> {
        unsafe {
            Guard {
                value: Box::new((*(*self)).clone()),
                inner: &mut *self.inner.get(),
            }
        }
    }
    /// Free all old versions of the data.  Because this method
    /// requires a mutable reference, it is guaranteed that no
    /// references exist.
    pub fn clean(&mut self) {
        if self.have_borrowed.load(Ordering::Acquire) {
            unsafe {
                let mut inner = &mut *self.inner.get();
                inner.borrow_count -= 1;
                if inner.borrow_count == 0 {
                    inner.old = Vec::new();
                }
            }
            self.have_borrowed.store(false, Ordering::Release);
        }
    }
}

impl<T> std::ops::Deref for RcCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let aleady_borrowed = self.have_borrowed.load(Ordering::Acquire);
        unsafe {
            let mut inner = &mut *self.inner.get();
            if !aleady_borrowed {
                inner.borrow_count += 1;
                self.have_borrowed.store(true, Ordering::Release); // indicate we have borrowed this once.
            }
            &inner.current
        }
    }
}

impl<T> std::borrow::Borrow<T> for RcCell<T> {
    fn borrow(&self) -> &T {
        &*self
    }
}

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

#[derive(Clone)]
pub struct ArcCell<T> {
    inner: Arc<UnsafeCell<BoxCellInner<T>>>,
    have_borrowed: Cell<bool>,
}
// unsafe impl<T: Send + Clone> Send for ArcCell<T> {}
// unsafe impl<T: Sync + Clone> Sync for ArcCell<T> {}

impl<T: Clone> ArcCell<T> {
    pub fn new(value: T) -> ArcCell<T> {
        ArcCell {
            inner: Arc::new(UnsafeCell::new(BoxCellInner {
                current: Box::new(value),
                old: Vec::new(),
                borrow_count: 0,
            })),
            have_borrowed: Cell::new(false),
        }
    }
    /// Make a copy of the data and return a reference.
    ///
    /// When the guard is dropped, `self` will be updated.  There is
    /// no protection against two simultaneous updates.  The one that
    /// drops second will "win".
    pub fn update<'a>(&'a self) -> impl 'a + std::ops::DerefMut<Target=T> {
        unsafe {
            Guard {
                value: Box::new((*(*self)).clone()),
                inner: &mut *self.inner.get(),
            }
        }
    }
    /// Free all old versions of the data.  Because this method
    /// requires a mutable reference, it is guaranteed that no
    /// references exist.
    pub fn clean(&mut self) {
        if self.have_borrowed.get() {
            self.have_borrowed.set(false);
            unsafe {
                let mut inner = &mut *self.inner.get();
                inner.borrow_count -= 1;
                if inner.borrow_count == 0 {
                    inner.old = Vec::new();
                }
            }
        }
    }
}

impl<T> std::ops::Deref for ArcCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let aleady_borrowed = self.have_borrowed.get();
        self.have_borrowed.set(true); // indicate we have borrowed this once.
        unsafe {
            let mut inner = &mut *self.inner.get();
            if !aleady_borrowed {
                inner.borrow_count += 1;
            }
            &inner.current
        }
    }
}

impl<T> std::borrow::Borrow<T> for ArcCell<T> {
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
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// ```
pub struct BoxCell<T> {
    inner: UnsafeCell<BoxCellInner<T>>,
}
pub struct BoxCellInner<T> {
    current: Box<T>,
    old: Vec<Box<T>>,
    borrow_count: usize,
}

impl<T: Clone> BoxCell<T> {
    pub fn new(value: T) -> BoxCell<T> {
        BoxCell {
            inner: UnsafeCell::new(BoxCellInner {
                current: Box::new(value),
                old: Vec::new(),
                borrow_count: 0,
            }),
        }
    }
    /// Make a copy of the data and return a reference.
    ///
    /// When the guard is dropped, `self` will be updated.  There is
    /// no protection against two simultaneous writes.  The one that
    /// drops second will "win".
    pub fn update<'a>(&'a self) -> impl 'a + std::ops::DerefMut<Target=T> {
        unsafe {
            Guard {
                value: Box::new((*self).clone()),
                inner: &mut *self.inner.get(),
            }
        }
    }
    /// Free all old versions of the data.  Because this method
    /// requires a mutable reference, it is guaranteed that no
    /// references exist.
    pub fn clean(&mut self) {
        unsafe {
            (*self.inner.get()).old = Vec::new();
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
            let b = &mut (*self.inner.get());
            b.old = Vec::new();
            b.current.borrow_mut()
        }
    }
}

impl<T> std::ops::Deref for BoxCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            &(*self.inner.get()).current
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
