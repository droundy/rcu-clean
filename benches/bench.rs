use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::cell::RefCell;

use unguarded::{BoxRcu, RcRcu, ArcRcu};

use criterion::{Criterion, criterion_group, criterion_main};

type RcRefCell<T> = Rc<RefCell<T>>;
type BoxRefCell<T> = Box<RefCell<T>>;
type BoxMutex<T> = Box<Mutex<T>>;
type BoxRwLock<T> = Box<RwLock<T>>;
type ArcMutex<T> = Arc<Mutex<T>>;
type ArcRwLock<T> = Arc<RwLock<T>>;

macro_rules! benchme {
    ($name:expr, $t:ident, $new:expr, $deref:expr) => {
        criterion::Fun::new($name, |b,&n_to_copy| {
            b.iter_with_setup(|| {
                let mut data: Vec<$t<usize>> = Vec::new();
                for i in 0..n_to_copy {
                    data.push($new(i));
                }
                let mut total: usize = 0;
                for x in data.iter() {
                    total += $deref(x);
                }
                data[0] = $new(total);
                (data, total)
            }, |(data, truetot)| {
                let mut total: usize = 0;
                for x in data.iter() {
                    total += $deref(x);
                }
                assert_eq!(total, truetot*2);
            });
        })
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut funs: Vec<criterion::Fun<usize>> = Vec::new();
    funs.push(benchme!("BoxRefCell", BoxRefCell, |a| Box::new(RefCell::new(a)),
                       |x: &BoxRefCell<usize>| -> usize { *x.borrow() }));
    funs.push(benchme!("RcRefCell", RcRefCell, |a| Rc::new(RefCell::new(a)),
                       |x: &RcRefCell<usize>| -> usize { *x.borrow() }));

    funs.push(benchme!("BoxMutex", BoxMutex, |a| Box::new(Mutex::new(a)),
                       |x: &BoxMutex<usize>| -> usize { *x.lock().unwrap() }));
    funs.push(benchme!("ArcMutex", ArcMutex, |a| Arc::new(Mutex::new(a)),
                       |x: &ArcMutex<usize>| -> usize { *x.lock().unwrap() }));

    funs.push(benchme!("BoxRwLock", BoxRwLock, |a| Box::new(RwLock::new(a)),
                       |x: &BoxRwLock<usize>| -> usize { *x.read().unwrap() }));
    funs.push(benchme!("ArcRwLock", ArcRwLock, |a| Arc::new(RwLock::new(a)),
                       |x: &ArcRwLock<usize>| -> usize { *x.read().unwrap() }));

    funs.push(benchme!("Box", Box, Box::new, |x: &Box<usize>| **x));
    funs.push(benchme!("Rc", Rc, Rc::new, |x: &Rc<usize>| **x));
    funs.push(benchme!("Arc", Arc, Arc::new, |x: &Arc<usize>| **x));


    funs.push(benchme!("RcRcu", RcRcu, RcRcu::new,
                       |x: &RcRcu<usize>| **x as usize));
    funs.push(benchme!("ArcRcu", ArcRcu, ArcRcu::new,
                       |x: &ArcRcu<usize>| **x as usize));
    funs.push(benchme!("BoxRcu", BoxRcu, BoxRcu::new,
                       |x: &BoxRcu<usize>| **x as usize));

    funs.reverse();
    c.bench_functions("sum", funs, 1000);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
