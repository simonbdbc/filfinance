use ordered_local_logger::*;
use std::{
    sync::{Arc, Barrier},
    time::{Duration, Instant},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Runtime::new(Config::new(
        std::env::temp_dir().join("ordered-noisy-instance"),
    ))?;
    let loud = rt.instance(InstanceConfig::new("loud"))?;
    let quiet = rt.instance(InstanceConfig::new("quiet"))?;
    let gate = Arc::new(Barrier::new(2));
    let g = gate.clone();
    let thread = std::thread::spawn(move || {
        g.wait();
        for i in 0..500000 {
            info!(loud, "loud {i}");
        }
    });
    let mut samples = Vec::with_capacity(1000);
    gate.wait();
    for i in 0..1000 {
        let start = Instant::now();
        info!(quiet, "quiet {i}");
        samples.push(start.elapsed().as_nanos() as u64);
        std::thread::sleep(Duration::from_millis(1));
    }
    thread.join().unwrap();
    let r = rt.shutdown()?;
    samples.sort_unstable();
    let q = &r.instances["quiet"];
    let l = &r.instances["loud"];
    println!(
        "{{\"quiet_accepted\":{},\"quiet_refused\":{},\"loud_accepted\":{},\"loud_refused\":{},\"quiet_p99_ns\":{},\"pending\":{}}}",
        q.accepted,
        q.refused(),
        l.accepted,
        l.refused(),
        samples[990],
        r.pending
    );
    Ok(())
}
