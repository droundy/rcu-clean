use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;
use crate::RCU;

/// An threadsafe owned pointer that allows interior mutability
///
/// The [BoxCellSync] is functionally equivalent to `Box<Mutex<T>>`,
/// except that reads (of the old value) may happen while a write is
/// taking place.
///
/// Read access using `[BoxCellSync]` is an atomic pointer read..
///
/// The main thing that simplifies and speeds up `[BoxCellSync]` is that
/// it cannot be either cloned or shared across threads.  Thus we
/// don't need any fancy locking or use of atomics.
///
/// ```
/// let x = unguarded::BoxCellSync::new(3);
/// let y: &usize = &(*x);
/// *x.update() = 7; // Wow, we are mutating something we have borrowed!
/// assert_eq!(*y, 3); // the old reference is still valid.
/// assert_eq!(*x, 7); // but the pointer now points to the new value.
/// ```
pub struct BoxCellSync<T> {
    current: AtomicPtr<T>,
    old: Mutex<Vec<Box<T>>>,
}

impl<'a,T: 'a> RCU<'a> for BoxCellSync<T> {
    type Target = T;
    type OldGuard = std::sync::MutexGuard<'a, Vec<Box<T>>>;
    fn get_raw(&self) -> *mut T {
        self.current.load(Ordering::Acquire)
    }
    fn set_raw(&self, new: *mut T) -> *mut T {
        self.current.swap(new, Ordering::Release)
    }
    fn get_old_guard(&'a self) -> Self::OldGuard {
        self.old.lock().unwrap()
    }
}
crate::impl_rcu!(BoxCellSync);

impl<T: Clone> BoxCellSync<T> {
    pub fn new(value: T) -> BoxCellSync<T> {
        BoxCellSync {
            current: AtomicPtr::new(Box::into_raw(Box::new(value))),
            old: Mutex::new(Vec::new()),
        }
    }
    /// Free all old versions of the data.  Because this method
    /// requires a mutable reference, it is guaranteed that no
    /// references exist.
    pub fn clean(&mut self) {
        *self.old.lock().unwrap() = Vec::new();
    }
}

impl<T: Clone> std::borrow::BorrowMut<T> for BoxCellSync<T> {
    fn borrow_mut(&mut self) -> &mut T {
        // Since we are mut, we know there are no other borrows out
        // there, so we can throw away any old references.  We also
        // don't need to bother cloning the data, since (again) there
        // are no other references out there.  Yay.
        self.clean();
        // I think it's safe to use a relaxed load here because we
        // have exclusive access to this BoxCellSync, so there must be
        // some other synchronization done.
        unsafe { &mut *self.current.load(Ordering::Acquire) }
    }
}
