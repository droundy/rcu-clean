use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;

use unguarded::{BoxCell, BoxCellSync, RcCell, ArcCell};

use criterion::{Criterion, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let mut funs = vec![criterion::Fun::new("Box", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Box<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Box::new(i));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    })];
    funs.push(criterion::Fun::new("Rc", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Rc<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Rc::new(i));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Arc", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Arc<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Arc::new(i));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Box<RefCell>", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Box<RefCell<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Box::new(RefCell::new(i)));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += *x.borrow();
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Rc<RefCell>", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Rc<RefCell<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Rc::new(RefCell::new(i)));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += *x.borrow();
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Box<Mutex>", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Box<Mutex<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Box::new(Mutex::new(i)));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += *x.lock().unwrap();
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Arc<Mutex>", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Arc<Mutex<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Arc::new(Mutex::new(i)));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += *x.lock().unwrap();
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("BoxCell", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<BoxCell<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(BoxCell::new(i));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("fresh RcCell", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<RcCell<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(RcCell::new(i));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("RcCell", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<RcCell<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(RcCell::new(i));
            }
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            data[0] = RcCell::new(total);
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("fresh BoxCellSync", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<BoxCellSync<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(BoxCellSync::new(i));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("BoxCellSync", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<BoxCellSync<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(BoxCellSync::new(i));
            }
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            data[0] = BoxCellSync::new(total);
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("fresh ArcCell", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<ArcCell<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(ArcCell::new(i));
            }
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("ArcCell", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<ArcCell<usize>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(ArcCell::new(i));
            }
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            data[0] = ArcCell::new(total);
            data
        }, |data| {
            let mut total: usize = 0;
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.reverse();
    c.bench_functions("sum", funs, 1000);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
