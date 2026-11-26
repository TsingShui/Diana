use super::PlatformCache;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

#[test]
fn concurrent_platform_loads_publish_once_after_a_retryable_error() {
    let cache = PlatformCache::new();
    let attempts = AtomicUsize::new(0);
    let first = cache.get_or_load(|| {
        attempts.fetch_add(1, Ordering::Relaxed);
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "incomplete index",
        ))
    });
    assert_eq!(first.unwrap_err().kind(), io::ErrorKind::InvalidData);
    let start = Barrier::new(16);
    std::thread::scope(|scope| {
        let threads = (0..16)
            .map(|_| {
                scope.spawn(|| {
                    start.wait();
                    cache
                        .get_or_load(|| {
                            attempts.fetch_add(1, Ordering::Relaxed);
                            Ok(9u32)
                        })
                        .unwrap()
                })
            })
            .collect::<Vec<_>>();
        let values = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>();
        assert!(values.iter().all(|value| Arc::ptr_eq(&values[0], value)));
        assert_eq!(*values[0], 9);
    });
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
}
