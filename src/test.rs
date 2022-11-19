use crate::Lineage;
use std::{sync::Arc, thread, time::Duration};

#[test]
fn t001() {
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
fn t002() {
    let l: Arc<Lineage<String>> = Arc::new(Lineage::new("1".into()));

    let mut threads = Vec::new();
    for _ in 0..10 {
        let l = Arc::clone(&l);
        threads.push(thread::spawn(move || {
            for _ in 0..10 {
                thread::sleep(Duration::from_millis(1));

                for _ in 0..10 {
                    l.set(l.get().clone());
                }
            }
        }));
    }

    for t in threads {
        let _ = t.join();
    }
}

#[test]
fn t003() {
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
fn t004() {
    for i in 0..4 {
        let l: Lineage<String> = Lineage::new("0".into());
        for j in 0..i {
            l.set(format!("{}", j + 1));
        }

        l.into_inner();
    }
}
