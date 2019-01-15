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
//! 1. `[BoxRcu]` is an owned pointer similar to [std::box::Box].  If
//!    you like, it is actually closer to `Box<RefCell<T>>`, or even
//!    `Box<Mutex<T>>`, but without the nuisance of having to call
//!    borrow when reading.
//!
//! 2. `[RcRcu]` is a reference counted pointer similar to [std::rc::Rc].
//!    If you like, it is actually closer to `Rc<RefCell<T>>`, but
//!    without the nuisance of having to call borrow when reading.
//!
//! 3. `ArcRcu` is planned to be a thread-safe reference counted
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
//! for extra convenience (and much improved read speed in the case of
//! `ArcRcu`) on the read operations.  You have two options for how to
//! handle this.
//!
//! One option is to simply store those extra copies until then entire
//! smart pointer itself is freed.  That is what happens if you do
//! nothing, and for small data that is only mutated once, it's a fine
//! option.  However, for `[ArcRcu]` and `[RcRcu]` there will be a
//! slowdown on reading until you do call clean, since an extra level
//! of pointer redirection will be required.
//!
//! The other option is to call `clean()` when convenient.  `clean`
//! takes a `&mut self`, so when it is called, the compiler will prove
//! to us that there are no other references out there via *this*
//! smart pointer.  For `BoxCell` that is sufficient to prove that we
//! can free the data.  In the case of the reference counted data
//! pointers, we keep track of a count of how many copies have been
//! dereferenced since the last time `clean` was called.  We could
//! probably be more accurate with "epoch" tracking, but I don't know
//! that the complexity will be worthwhile.

mod boxrcu;
pub use crate::boxrcu::BoxRcu;

mod rcrcu;
pub use crate::rcrcu::RcRcu;

mod arcrcu;
pub use crate::arcrcu::ArcRcu;

macro_rules! impl_stuff {
    ($t:ident) => {
        impl<T: PartialEq> PartialEq for $t<T> {
            fn eq(&self, other: &$t<T>) -> bool {
                &(**self) == &(**other)
            }
        }
        impl<T: Eq> Eq for $t<T> {}
        impl<T: PartialOrd> PartialOrd for $t<T> {
            fn partial_cmp(&self, other: &$t<T>) -> Option<std::cmp::Ordering> {
                (**self).partial_cmp(&**other)
            }
        }
        impl<T: Ord> Ord for $t<T> {
            fn cmp(&self, other: &$t<T>) -> std::cmp::Ordering {
                (**self).cmp(&**other)
            }
        }
        impl<T: std::fmt::Debug> std::fmt::Debug for $t<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                (**self).fmt(f)
            }
        }
        impl<T: std::fmt::Display> std::fmt::Display for $t<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                (**self).fmt(f)
            }
        }
        #[cfg(serde)]
        impl<T: serde::Serialize> serde::Serialize for $t<T> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                (*self).serialize(serializer)
            }
        }
        #[cfg(serde)]
        impl<'de, T:  Deserialize<'de>> Deserialize<'de> for $t<T> {
            fn deserialize<D: Deserializer<'de>>(deserializer: D)
                                                 -> Result<Self, D::Error>
            {
                T::deserialize(deserializer).map(|v| $t::new(v))
            }
        }
    }
}

impl_stuff!(BoxRcu);
impl_stuff!(RcRcu);
impl_stuff!(ArcRcu);
