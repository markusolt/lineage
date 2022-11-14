use crate::Lineage;
use std::{sync::Arc, thread, time::Duration};

#[test]
fn t001() {
    let mut l: Lineage<String, 0> = Lineage::new("1".into());

    {
        let v1 = l.get();
        l.set("2".into());
        let v2 = l.get();
        l.set("3".into());
        let v3 = l.get();

        assert!(v1 == "1");
        assert!(v2 == "2");
        assert!(v3 == "3");
    }

    l.clear();
    assert!(l.get() == "3");

    l.set_mut("4".into());
    assert!(l.get() == "4");

    l.set_mut("5".into());
    assert!(l.get() == "5");
}

#[test]
fn t002() {
    let mut l: Lineage<String, 32> = Lineage::new("1".into());

    {
        let v1 = l.get();
        l.set("2".into());
        let v2 = l.get();
        l.set("3".into());
        let v3 = l.get();

        assert!(v1 == "1");
        assert!(v2 == "2");
        assert!(v3 == "3");
    }

    l.clear();
    assert!(l.get() == "3");

    l.set_mut("4".into());
    assert!(l.get() == "4");

    l.set_mut("5".into());
    assert!(l.get() == "5");
}

#[test]
fn t003() {
    let l: Lineage<String, 3> = Lineage::new("1".into());

    let v1 = l.get();
    l.set("2".into());
    let v2 = l.get();
    l.set("3".into());
    let v3 = l.get();
    l.set("4".into());
    let v4 = l.get();
    l.set("5".into());
    let v5 = l.get();
    l.set("6".into());
    let v6 = l.get();

    assert!(v1 == "1");
    assert!(v2 == "2");
    assert!(v3 == "3");
    assert!(v4 == "4");
    assert!(v5 == "5");
    assert!(v6 == "6");
}

#[test]
fn t004() {
    let l: Arc<Lineage<String, 0>> = Arc::new(Lineage::new("1".into()));

    let mut threads = Vec::new();
    for _ in 0..20 {
        let l = Arc::clone(&l);
        threads.push(thread::spawn(move || {
            for _ in 0..20 {
                thread::sleep(Duration::from_millis(1));

                for _ in 0..30 {
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
fn t005() {
    let l: Arc<Lineage<String, 32>> = Arc::new(Lineage::new("1".into()));

    let mut threads = Vec::new();
    for _ in 0..20 {
        let l = Arc::clone(&l);
        threads.push(thread::spawn(move || {
            for _ in 0..20 {
                thread::sleep(Duration::from_millis(1));

                for _ in 0..30 {
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
fn t006() {
    let mut l: Lineage<usize, 10> = Lineage::new(0);

    for i in 0..20 {
        for j in 0..i {
            l.set(j);
        }

        l.clear();
    }

    for i in 0..20 {
        for j in 0..i {
            l.set_mut(j);
        }

        l.clear();
    }
}
