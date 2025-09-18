use std::thread;
use std::time::Duration;

fn main() {
    let handle = thread::spawn(|| {
        for i in 1..10 {
            println!("In thread: {i}");
            thread::sleep(Duration::from_millis(1));
        }
    });

    for i in 1..5 {
        println!("In main thread: {i}");
        thread::sleep(Duration::from_millis(3));
    }

    let res = handle.join();
    assert!(res.is_ok());

    let v = vec![1, 2, 3];

    let handle = thread::spawn(|| {
        let v = v; // Inner v moves the outer v.
        println!("Here's a vector: {:?}", v);
    });

    assert!(handle.join().is_ok());
}
