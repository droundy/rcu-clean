use unguarded::{BoxCell, BoxCellSync, RcCell, ArcCell, RcNew};

macro_rules! testrc {
    ($name:ident, $t:ident) => {
        #[test]
        fn $name() {
            let orig = $t::new((4,4));
            let copy: $t<(usize,usize)> = orig.clone();
            assert_eq!(*orig, *copy);
        }
    }
}

// testrc!(rccell, RcCell);

testrc!(rcnew, RcNew);

// testrc!(arccell, ArcCell);
