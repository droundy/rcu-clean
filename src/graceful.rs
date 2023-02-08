//! An attempt at Rcu with grace periods
//!
//! We can allocate `Rcu` just like you would a `Arc`.
//! ```
//! let v = rcu_clean::graceful::Rcu::new("hello");
//! ```
//! These pointers are freed just like an ordinary `Arc`.  The big difference is
//! that you can update these pointers while they are being read, but reading
//! from the pointers requires a [`Grace`].
//! ```
//! let v = rcu_clean::graceful::Rcu::new(vec![1,2,3,4]);
//! {
//!     let grace = rcu_clean::graceful::Grace::new();
//!     let read = v.read(&grace);
//!     let mut iter = read.iter();
//!     // Demonstrate that we can start iterating through our vec.
//!     assert_eq!(Some(&1), iter.next());
//!     // Now let's modify the vec.
//!     v.update(|v| v.push(5));
//!     assert_eq!(4, read.len()); // `read` still refers to the 4-element vec.
//!     assert_eq!(Some(&2), iter.next()); // the iterator is still working.
//!     assert_eq!(5, v.read(&grace).len()); // a new read gets the updated value
//! }
//! // At this point the 4-element vec will be freed, since the grace period is over.
//! ```
//!
//! The fanciness of this approach is that while the `Grace` costs something to
//! create and drop, the [`ReadGuard`] created by `v.read()` costs nothing
//! either to create or to drop, so reads are literally free (on strongly
//! ordered machines) once you enter a grace period.  Technically the reads cost
//! a `AtomicPtr::load(Ordering::Acquire)`, which might be more expensive than a
//! pointer read, but should not be much so, and should be far cheaper than a
//! `RwLock::read` which would be the `std` alternative for a data structure
//! with many readers and few writers.
use std::ops::Deref;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, Mutex};

use once_cell::sync::OnceCell;

/// A reference-counted RCU pointer with grace periods
pub struct Rcu<T>(AtomicPtr<T>);

unsafe impl<T: Sync> Sync for Rcu<T> {}
unsafe impl<T: Send> Send for Rcu<T> {}

impl<T> Clone for Rcu<T> {
    fn clone(&self) -> Self {
        let p = self.0.swap(std::ptr::null_mut(), Ordering::Acquire);
        let arc: Arc<T> = unsafe { Arc::from_raw(p) };
        let other = arc.clone();
        let _copy_i_am_keeping = Arc::into_raw(arc);
        // Use the other copy for the clone
        Rcu(AtomicPtr::new(Arc::into_raw(other) as *mut T))
    }
}

impl<T> Drop for Rcu<T> {
    fn drop(&mut self) {
        let p = self.0.swap(std::ptr::null_mut(), Ordering::Acquire);
        let _to_free = unsafe { Arc::from_raw(p) };
    }
}

impl<T> From<Arc<T>> for Rcu<T> {
    fn from(b: Arc<T>) -> Self {
        Rcu(std::sync::atomic::AtomicPtr::new(Arc::into_raw(b) as *mut T))
    }
}

impl<T: Clone + Send + Sync + 'static> Rcu<T> {
    /// Allocate a new Rcu pointer
    ///
    /// This is no more expensive than `Arc::new`.
    pub fn new(value: T) -> Self {
        Self::from(Arc::new(value))
    }
    /// Read the pointer, with the given grace period
    ///
    /// This method is just an atomic pointer load with acquire ordering, and is
    /// thus quite cheap.  It returns an [`RcuGuard`] which cannot outlive the
    /// grace period.  The guard implements `Deref` that is a noop, so overall
    /// the cost of reading from an `Rcu` is just the cost of a single atomic
    /// pointer load (and then of course dereferencing that pointer).
    pub fn read<'a, 'b: 'a>(&'b self, _grace: &'a Grace) -> RcuGuard<'a, T> {
        let p = self.0.load(Ordering::Acquire);
        RcuGuard {
            ptr: unsafe { &*p },
        }
    }
    /// Modify the contents of the `Rcu`.
    ///
    /// This method reads and copies the value of the `Rcu`, and then calls your
    /// closure (or function) to update the value.  Once the new value has been
    /// computed, the `Rcu` is atomically updated to point to the new values.
    ///
    /// The old value will be retained until the last `Grace` that is open when
    /// we start the `update` is dropped.
    ///
    /// Note that simultaneous updates to the same pointer are possible and are
    /// *safe* but are not *recommended*.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        // start by getting the grace period for our copy.
        let mut new = Arc::new(self.read(&Grace::new()).clone());
        f(Arc::get_mut(&mut new).unwrap());

        // Now we take the grace-period lock before doing our update.  Since we
        // have just source of grace, this means no other critical update
        // sections are ongoing, and all updates are totally ordered.
        //
        // It also means that no one can start a new grace period while we're
        // working on this change.
        let mut lock = GRACE.get().unwrap().0.lock().unwrap();

        let mut vec_lock = lock.lock().unwrap();

        // First we store the old value to be freed.
        let old = self.0.swap(Arc::into_raw(new) as *mut T, Ordering::Release);
        vec_lock.push(unsafe { Arc::from_raw(old) });

        let next_grace = Arc::new(Mutex::new(Vec::new()));
        // The old grace period will depend on the new grace period, so the
        // freeing happens in the correct order.
        vec_lock.push(Arc::new(next_grace.clone()));
        drop(vec_lock);

        // Now we update the SourceOfGrace, which should always hold an empty
        // vector, so that everything that does need to get freed *will* get
        // freed.
        *lock = next_grace;
    }
}

static GRACE: OnceCell<SourceOfGrace> = OnceCell::new();

/// A grace period
///
/// The grace period determines how long Rcu values must be retained to ensure
/// that no reader ends up reading after free.  Creating a `Grace` is
/// relatively expensive, so ideally you'd like to create a single `Grace` and
/// using it for a number of reads.
#[derive(Clone)]
pub struct Grace {
    _to_free: Arc<Mutex<Vec<Arc<dyn Send + Sync>>>>,
}

impl Grace {
    /// Create a new grace period
    ///
    /// This grace period will allow you to perform multiple reads, and be
    /// confident that no Rcu data that was accessible to these reads will be
    /// freed until after this `Grace` has been dropped.
    pub fn new() -> Grace {
        Grace {
            _to_free: GRACE
                .get_or_init(|| SourceOfGrace(Mutex::new(Arc::new(Mutex::new(Vec::new())))))
                .0
                .lock()
                .unwrap()
                .clone(),
        }
    }
}

struct SourceOfGrace(Mutex<Arc<Mutex<Vec<Arc<dyn Send + Sync>>>>>);

/// A reference to contents that are being read
///
/// Note that the `RcuGuard` really just holds a reference, and its `Deref`
/// costs no cpu instructions.
pub struct RcuGuard<'a, T> {
    ptr: &'a T,
}
impl<'a, T> Deref for RcuGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}
