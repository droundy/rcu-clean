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

mod boxcell;
pub use crate::boxcell::BoxCell;

mod boxcellsync;
pub use crate::boxcellsync::BoxCellSync;

mod rccell;
pub use crate::rccell::RcCell;

mod rcnew;
pub use crate::rcnew::RcNew;

mod arccell;
pub use crate::arccell::ArcCell;

mod arcnew;
pub use crate::arcnew::ArcNew;

pub trait RCU<'a> {
    type Target: 'a;
    type OldGuard: 'a + std::ops::DerefMut<Target=Vec<Box<Self::Target>>>;
    fn get_raw(&self) -> *mut Self::Target;
    /// Set a new value, and return the old one.
    fn set_raw(&self, new: *mut Self::Target) -> *mut Self::Target;
    fn get_old_guard(&'a self) -> Self::OldGuard;

    /// Release any references that we might have taken.  Return true
    /// if the old data may be freed.
    fn release(&mut self) -> bool {
        true
    }
}

#[macro_export]
macro_rules! impl_rcu {
    ($t:ident) => {
        impl<T> std::ops::Deref for $t<T> {
            type Target = T;
            fn deref(&self) -> &T {
                unsafe { &*self.get_raw() }
            }
        }
        impl<T> std::borrow::Borrow<T> for $t<T> {
            fn borrow(&self) -> &T {
                &*self
            }
        }
        impl<T> Drop for $t<T> {
            fn drop(&mut self) {
                // this frees the current value of the pointer.  It is acquire
                // because the contents of the pointer must be up to date so
                // the drop of T doesn't do anything crazy.
                unsafe { Box::from_raw(self.get_raw()); }
            }
        }

        impl<T: Clone> $t<T> {
            /// Make a copy of the data and return a mutable guarded reference.
            ///
            /// When the guard is dropped, `self` will be updated.
            pub fn update<'a>(&'a self) -> impl 'a + std::ops::DerefMut<Target=T> {
                Guard {
                    value: Box::into_raw(Box::new((*(*self)).clone())),
                    ptr: self,
                    guard: self.get_old_guard(),
                }
            }
            /// Free all old versions of the data if possible.  Because this
            /// method requires a mutable reference, it is guaranteed that no
            /// references exist to this particular smart pointer.
            pub fn clean(&mut self) {
                if self.release() {
                    *self.get_old_guard() = Vec::new();
                }
            }
        }

        struct Guard<'a,T: Clone> {
            value: *mut T,
            ptr: &'a $t<T>,
            guard: < $t<T> as RCU<'a> >::OldGuard,
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
                let oldvalue = self.ptr.set_raw(self.value);
                self.guard.push(unsafe { Box::from_raw(oldvalue) });
            }
        }
    }
}
