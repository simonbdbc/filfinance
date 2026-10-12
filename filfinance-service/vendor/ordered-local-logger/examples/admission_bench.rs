//! Compare serial mutex+deque with serial mutex+bounded channel (same order).
use std::{
    collections::VecDeque,
    sync::{Arc, Barrier, Mutex, mpsc},
    time::Instant,
};
fn main() {
    for producers in [1, 2, 4, 8] {
        for channel in [false, true] {
            let gate = Arc::new(Barrier::new(producers + 1));
            let queue = Arc::new(Mutex::new(VecDeque::with_capacity(1000000)));
            let (tx, rx) = mpsc::sync_channel(1000000);
            let serial = Arc::new(Mutex::new(tx));
            let mut threads = Vec::new();
            for _ in 0..producers {
                let gate = gate.clone();
                let queue = queue.clone();
                let serial = serial.clone();
                threads.push(std::thread::spawn(move || {
                    let mut samples = Vec::with_capacity(20000);
                    gate.wait();
                    for index in 0..20000 {
                        let t = Instant::now();
                        if channel {
                            serial.lock().unwrap().try_send(index).unwrap();
                        } else {
                            queue.lock().unwrap().push_back(index);
                        }
                        samples.push(t.elapsed().as_nanos() as u64);
                    }
                    samples
                }));
            }
            gate.wait();
            let mut all = Vec::new();
            for thread in threads {
                all.extend(thread.join().unwrap());
            }
            all.sort_unstable();
            println!(
                "{{\"primitive\":\"{}\",\"producers\":{},\"p50_ns\":{},\"p99_ns\":{},\"p999_ns\":{}}}",
                if channel {
                    "mutex_channel"
                } else {
                    "mutex_deque"
                },
                producers,
                all[all.len() / 2],
                all[all.len() * 99 / 100],
                all[all.len() * 999 / 1000]
            );
            drop(rx);
        }
    }
}
