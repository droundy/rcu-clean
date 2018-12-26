use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;

use unguarded::{BoxCell, BoxCellSync, RcCell, ArcCell, RcNew};

use criterion::{Criterion, criterion_group, criterion_main};

type RcRefCell<T> = Rc<RefCell<T>>;
type BoxRefCell<T> = Box<RefCell<T>>;
type BoxMutex<T> = Box<Mutex<T>>;
type ArcMutex<T> = Arc<Mutex<T>>;

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
                data
            }, |data| {
                let mut total: usize = 0;
                for x in data.iter() {
                    total += $deref(x);
                }
                total
            });
        })
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut funs: Vec<criterion::Fun<usize>> = Vec::new();
    funs.push(benchme!("Box", Box, Box::new, |x: &Box<usize>| **x));
    funs.push(benchme!("Rc", Rc, Rc::new, |x: &Rc<usize>| **x));
    funs.push(benchme!("Arc", Arc, Arc::new, |x: &Arc<usize>| **x));

    funs.push(benchme!("BoxRefCell", BoxRefCell, |a| Box::new(RefCell::new(a)),
                       |x: &BoxRefCell<usize>| -> usize { *x.borrow() }));
    funs.push(benchme!("RcRefCell", RcRefCell, |a| Rc::new(RefCell::new(a)),
                       |x: &RcRefCell<usize>| -> usize { *x.borrow() }));

    funs.push(benchme!("BoxMutex", BoxMutex, |a| Box::new(Mutex::new(a)),
                       |x: &BoxMutex<usize>| -> usize { *x.lock().unwrap() }));
    funs.push(benchme!("ArcMutex", ArcMutex, |a| Arc::new(Mutex::new(a)),
                       |x: &ArcMutex<usize>| -> usize { *x.lock().unwrap() }));

    funs.push(benchme!("BoxCellSync", BoxCellSync, BoxCellSync::new,
                       |x: &BoxCellSync<usize>| **x));
    funs.push(benchme!("BoxCell", BoxCell, BoxCell::new,
                       |x: &BoxCell<usize>| **x));
    funs.push(benchme!("ArcCell", ArcCell, ArcCell::new,
                       |x: &ArcCell<usize>| **x));
    funs.push(benchme!("RcCell", RcCell, RcCell::new,
                       |x: &RcCell<usize>| **x));
    funs.push(benchme!("RcNew", RcNew, RcNew::new,
                       |x: &RcNew<usize>| **x));

    funs.reverse();
    c.bench_functions("sum", funs, 1000);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
