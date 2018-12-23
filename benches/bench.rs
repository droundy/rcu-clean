use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;

use unguarded::{BoxCell, BoxCellSync, RcCell, ArcCell};

use criterion::{Criterion, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let mut funs = vec![criterion::Fun::new("Box", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Box<std::num::Wrapping<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Box::new(std::num::Wrapping(i)));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += **x;
            }
            total
        });
    })];
    funs.push(criterion::Fun::new("Rc", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Rc<std::num::Wrapping<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Rc::new(std::num::Wrapping(i)));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Arc", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Arc<std::num::Wrapping<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Arc::new(std::num::Wrapping(i)));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Box<RefCell>", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Box<RefCell<std::num::Wrapping<usize>>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Box::new(RefCell::new(std::num::Wrapping(i))));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += *x.borrow();
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Rc<RefCell>", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Rc<RefCell<std::num::Wrapping<usize>>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Rc::new(RefCell::new(std::num::Wrapping(i))));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += *x.borrow();
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Box<Mutex>", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Box<Mutex<std::num::Wrapping<usize>>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Box::new(Mutex::new(std::num::Wrapping(i))));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += *x.lock().unwrap();
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("Arc<Mutex>", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<Arc<Mutex<std::num::Wrapping<usize>>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(Arc::new(Mutex::new(std::num::Wrapping(i))));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += *x.lock().unwrap();
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("BoxCell", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<BoxCell<std::num::Wrapping<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(BoxCell::new(std::num::Wrapping(i)));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("RcCell", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<RcCell<std::num::Wrapping<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(RcCell::new(std::num::Wrapping(i)));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("BoxCellSync", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<BoxCellSync<std::num::Wrapping<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(BoxCellSync::new(std::num::Wrapping(i)));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.push(criterion::Fun::new("ArcCell", |b,&n_to_copy| {
        b.iter_with_setup(|| {
            let mut data: Vec<ArcCell<std::num::Wrapping<usize>>> = Vec::new();
            for i in 0..n_to_copy {
                data.push(ArcCell::new(std::num::Wrapping(i)));
            }
            data
        }, |data| {
            let mut total: std::num::Wrapping<usize> = std::num::Wrapping(0);
            for x in data.iter() {
                total += **x;
            }
            total
        });
    }));

    funs.reverse();
    c.bench_functions("sum", funs, 1<<10);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
