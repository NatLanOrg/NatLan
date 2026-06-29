// Bounded-concurrency map over a slice, using scoped OS threads. LLM stages are IO-bound
// (network round-trips), so running them concurrently turns "sum of all calls" into
// "slowest batch". Each call uses its own TCP connection, so there is no shared mutable state.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub fn max_workers() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 16)
}

// Apply `f` to each item with at most `workers` running at once, preserving input order.
pub fn par_map<T, R, F>(items: &[T], workers: usize, f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(usize, &T) -> R + Sync,
{
    let n = items.len();
    if n == 0 {
        return Vec::new();
    }
    let slots: Vec<Mutex<Option<R>>> = (0..n).map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let workers = workers.clamp(1, n);
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= n {
                    break;
                }
                let r = f(i, &items[i]);
                *slots[i].lock().unwrap() = Some(r);
            });
        }
    });
    slots.into_iter().map(|m| m.into_inner().unwrap().expect("slot filled")).collect()
}
