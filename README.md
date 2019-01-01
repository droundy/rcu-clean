# rcu-clean &emsp; [![Latest version](https://img.shields.io/crates/v/rcu-clean.svg)](https://crates.io/crates/rcu-clean) [![Documentation](https://docs.rs/rcu-clean/badge.svg)](https://docs.rs/rcu-clean)

This crate provides easy to use smart-pointers with interior
mutability.  These smart pointers use
[RCU](https://en.wikipedia.org/wiki/Read-copy-update) to allow
simultaneous reads and updates.  They implement `Deref` for reads,
which makes them both convenient (ergonomic) and fast on reads,
particularly for the `Arc` version that would otherwise require taking
a `Mutex` or `RwLock` in order to read the pointer.  The downside is
that old versions of the data are only freed when you have called the
`clean` method on each copy of the pointer.
