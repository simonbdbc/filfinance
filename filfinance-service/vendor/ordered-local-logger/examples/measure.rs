//! Reproducible emission samples. Run the release binary, not Cargo, under time(1).
use ordered_local_logger::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
    io::Write,
    sync::{Arc, Barrier},
    time::{Duration, Instant},
};
thread_local! {static TRACK:Cell<bool>=const {Cell::new(false)};static ALLOCS:Cell<u64>=const {Cell::new(0)};}
struct Counting;
// SAFETY: all operations delegate unchanged to the system allocator. TLS counters
// use const initialization, perform no allocation, and count only this thread.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACK.try_with(Cell::get).unwrap_or(false) {
            let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if TRACK.try_with(Cell::get).unwrap_or(false) {
            let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;
fn percentiles(values: &mut [u64]) -> String {
    values.sort_unstable();
    let q = |p: f64| {
        values
            .get(((values.len().saturating_sub(1)) as f64 * p).ceil() as usize)
            .copied()
            .unwrap_or(0)
    };
    format!(
        "{{\"n\":{},\"p50_ns\":{},\"p99_ns\":{},\"p999_ns\":{}}}",
        values.len(),
        q(0.5),
        q(0.99),
        q(0.999)
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let mode = args.get(1).map(String::as_str).unwrap_or("static");
    let producers = args.get(2).and_then(|n| n.parse().ok()).unwrap_or(1usize);
    let count = args
        .get(3)
        .and_then(|n| n.parse().ok())
        .unwrap_or(20000usize);
    let directory = args
        .get(4)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ordered-logger-measure"));
    let mut config = Config::new(directory);
    if let Some(ms) = args.get(5).and_then(|s| s.parse::<u64>().ok()) {
        config.group_duration = Duration::from_millis(ms);
    }
    let rt = Runtime::new(config)?;
    let mut c = InstanceConfig::new("bench");
    c.destinations = vec!["app".into(), "other".into()];
    let base = rt.instance(c)?;
    let l = base.with_context(&Fields::from([("workflow_id".into(), "measure".into())]))?;
    let gate = Arc::new(Barrier::new(producers + 1));
    let mut threads = Vec::new();
    eprintln!("RUN_PATH={}", rt.log_root().display());
    eprintln!("RUN_ID={}", rt.run_id());
    if args.iter().any(|v| v == "--interactive") {
        info!(l, "ready");
        rt.flush()?;
        eprintln!("READY: press Enter to measure");
        std::io::stderr().flush()?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
    }
    for worker in 0..producers {
        let logger = if mode == "tracing" {
            rt.instance(InstanceConfig::new(format!("trace-{worker}")))?
        } else {
            l.clone()
        };
        let base = base.clone();
        let gate = gate.clone();
        let mode = mode.to_owned();
        threads.push(std::thread::spawn(move || {
            let fields = Fields::from([
                ("worker_id".into(), Value::U64(worker as u64)),
                ("api".into(), "local".into()),
            ]);
            let mut accepted = Vec::with_capacity(count);
            let mut refused = Vec::with_capacity(count);
            let mut filtered = Vec::with_capacity(count);
            let mut allocations = [0u64; 3];
            #[cfg(feature = "tracing")]
            let _subscriber = {
                use tracing_subscriber::prelude::*;
                tracing::subscriber::set_default(
                    tracing_subscriber::registry().with(LoggerLayer::new(logger.clone())),
                )
            };
            gate.wait();
            for index in 0..count {
                let tracing_before = if mode == "tracing" {
                    logger.counters().accepted
                } else {
                    0
                };
                ALLOCS.with(|n| n.set(0));
                TRACK.with(|n| n.set(true));
                let start = Instant::now();
                let mut result = match mode.as_str() {
                    "filtered" => debug!(logger, "{}", black_box(index)),
                    "fields" => {
                        logger.emit(Level::Info, Source::default(), None, &fields, "fields")
                    }
                    "format" => info!(
                        logger,
                        "worker={} index={} value={:.5}",
                        worker,
                        index,
                        index as f64 / 3.0
                    ),
                    "context" => info!(logger, "context"),
                    "destinations" => base.emit(
                        Level::Info,
                        Source::default(),
                        Some(if index % 2 == 0 { "app" } else { "other" }),
                        &fields,
                        "destination",
                    ),
                    "nominal" => info!(logger, "sample worker={} index={}", worker, index),
                    #[cfg(feature = "tracing")]
                    "tracing" => {
                        tracing::info!(worker_id = worker as u64, index = index as u64, "tracing");
                        EmitResult::Accepted(0)
                    }
                    _ => info!(base, "static"),
                };
                let elapsed = start.elapsed().as_nanos() as u64;
                TRACK.with(|n| n.set(false));
                let a = ALLOCS.with(Cell::get);
                if mode == "tracing" && logger.counters().accepted == tracing_before {
                    result = EmitResult::Refused(Refusal::Full);
                }
                match result {
                    EmitResult::Accepted(_) => {
                        accepted.push(elapsed);
                        allocations[0] += a;
                    }
                    EmitResult::Refused(_) => {
                        refused.push(elapsed);
                        allocations[1] += a;
                    }
                    EmitResult::Filtered => {
                        filtered.push(elapsed);
                        allocations[2] += a;
                    }
                }
                if mode == "nominal" {
                    std::thread::sleep(Duration::from_micros(500));
                }
            }
            (accepted, refused, filtered, allocations)
        }));
    }
    let start = Instant::now();
    gate.wait();
    let (mut accepted, mut refused, mut filtered, mut allocations) =
        (Vec::new(), Vec::new(), Vec::new(), [0u64; 3]);
    for thread in threads {
        let (a, r, f, n) = thread.join().unwrap();
        accepted.extend(a);
        refused.extend(r);
        filtered.extend(f);
        for i in 0..3 {
            allocations[i] += n[i];
        }
    }
    let emission_seconds = start.elapsed().as_secs_f64();
    let report = rt.shutdown()?;
    let total_seconds = start.elapsed().as_secs_f64();
    println!(
        "{{\"mode\":\"{mode}\",\"producers\":{producers},\"count_per_producer\":{count},\"emission_seconds\":{emission_seconds},\"drain_seconds\":{total_seconds},\"accepted\":{},\"refused\":{},\"filtered\":{},\"producer_allocations\":{:?},\"written\":{},\"lines\":{},\"pending\":{},\"reserved_bytes\":{}}}",
        percentiles(&mut accepted),
        percentiles(&mut refused),
        percentiles(&mut filtered),
        allocations,
        report.totals.written,
        report.totals.lines,
        report.pending,
        report.reserved_bytes
    );
    assert_eq!(report.pending, 0);
    Ok(())
}
