use rcu_clean::{BoxRcu, RcRcu, ArcRcu};

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

macro_rules! testany {
    ($name:ident, $t:ident) => {
        #[test]
        fn $name() {
            let ptr = $t::new((4,4));
            assert_eq!(ptr.0, ptr.1);
            let foo = &ptr;
            assert_eq!(foo.0, 4);
            *ptr.update() = (5,5);
            assert_eq!(ptr.0, 5);
            assert_eq!(foo.0, 5);
            {
                let mut bar = ptr.update();
                assert_eq!(ptr.0, 5);
                assert_eq!(foo.0, 5);
                assert_eq!(bar.0, 5);
                bar.0 = 7;
                assert_eq!(ptr.0, 5);
                assert_eq!(foo.0, 5);
                assert_eq!(bar.0, 7);
            }
            assert_eq!(ptr.0, 7);
            assert_eq!(foo.0, 7);
        }
    }
}

testrc!(rcrcu, RcRcu);
testrc!(arcrcu, ArcRcu);

testany!(boxrcu_any, BoxRcu);
testany!(rcrcu_any, RcRcu);
testany!(arcrcu_any, ArcRcu);
