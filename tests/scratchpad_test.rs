use std::sync::{Arc, mpsc};
use std::thread;

#[test]
fn test_threads() {
    // Create a Multiple Producer, Single Consumer channel
    let (tx, rx) = mpsc::channel();
    let (txb, _rxb) = mpsc::channel();

    // Spawn the "listener" thread

    let handle = thread::spawn(move || {
        // rx.into_iter() creates a blocking iterator that yields
        // values until all senders are dropped.
        rx.into_iter().for_each(|message: Arc<String>| {
            println!("Thread received: {message}");
            txb.send(Box::new("received".to_owned())).unwrap();
        });

        println!("Channel closed. Thread exiting.");
        "finished"
    });

    // Send some data from the main thread
    let the_thing = Arc::new("Blabla".to_owned());

    tx.send(the_thing.clone()).unwrap();
    tx.send(the_thing.clone()).unwrap();

    // Dropping the sender allows the listener's iterator to finish
    drop(tx);

    // Wait for the thread to finish
    assert_eq!(handle.join().unwrap(), "finished");
}
