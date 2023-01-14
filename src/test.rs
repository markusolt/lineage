use crate::Lineage;
use std::{sync::Arc, thread, time::Duration};

#[test]
fn t001() {
    let _: Lineage<String> = Lineage::new("t001".into());
}

#[test]
fn t002() {
    let mut l: Lineage<String> = Lineage::new("1".into());

    let v1 = l.get();
    l.set("2".into());
    let v2 = l.get();
    l.set("3".into());
    let v3 = l.get();

    assert!(v1 == "1");
    assert!(v2 == "2");
    assert!(v3 == "3");

    assert!(l.get_mut() == "3");

    l.clear();
    assert!(l.get() == "3");

    l.set_mut("4".into());
    assert!(l.get() == "4");

    l.set_mut("5".into());
    assert!(l.get() == "5");

    assert!(l.into_inner() == "5");
}

#[test]
fn t003() {
    let mut l: Lineage<usize> = Lineage::new(5);
    assert!(*l.get_mut() == 5);
}

#[test]
fn t004() {
    for i in 0..4 {
        let l: Lineage<String> = Lineage::new("0".into());
        for j in 0..i {
            l.set(format!("{}", j + 1));
        }

        l.into_inner();
    }
}

#[test]
fn t005() {
    let mut l: Lineage<usize> = Lineage::new(0);

    for i in 0..10 {
        for j in 0..i {
            l.set(j);
        }

        l.clear();
    }

    for i in 0..10 {
        for j in 0..i {
            l.set_mut(j);
        }

        l.clear();
    }
}

#[test]
fn t006() {
    let l: Arc<Lineage<String>> = Arc::new(Lineage::new("t006".into()));

    let mut threads = Vec::new();
    for _ in 0..10 {
        let l = Arc::clone(&l);
        threads.push(thread::spawn(move || {
            for _ in 0..10 {
                thread::sleep(Duration::from_millis(1));

                for _ in 0..10 {
                    let val = l.get().clone();
                    l.set(val);
                }
            }
        }));
    }

    for t in threads {
        let _ = t.join();
    }
}

#[test]
fn t007() {
    const LEN: usize = 10;

    for i in 0..=LEN {
        let mut l: Lineage<String> = Lineage::new(0.to_string());
        for j in 0..LEN {
            l.set((j + 1).to_string());
        }

        let mut drain = l.drain();

        for j in 0..i {
            assert!(drain.size_hint().0 <= LEN - j);
            assert!(drain
                .size_hint()
                .1
                .map(|est| est >= LEN - j)
                .unwrap_or(true));

            assert!(drain.next() == Some((LEN - 1 - j).to_string()));
        }

        assert!(drain.size_hint().0 <= LEN - i);
        assert!(drain
            .size_hint()
            .1
            .map(|est| est >= LEN - i)
            .unwrap_or(true));

        if i == LEN {
            assert!(drain.next().is_none());
        }
    }
}
