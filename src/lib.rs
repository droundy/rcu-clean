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

mod boxrcu;
pub use crate::boxrcu::BoxRcu;

mod rcrcu;
pub use crate::rcrcu::RcRcu;

mod arcrcu;
pub use crate::arcrcu::ArcRcu;
