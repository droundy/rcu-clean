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

pub struct BoxCellInner<T> {
    current: Box<T>,
    old: Vec<Box<T>>,
    borrow_count: usize,
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
