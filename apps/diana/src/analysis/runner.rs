use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

use super::query::QueryError;

/// Each worker can queue one completed row; channel closure propagates sink
/// failure directly to producers.
enum Message<T> {
    Row(T),
    Failed(QueryError),
}

/// Runs `produce` over every image on a worker pool.
///
/// `context` is called once per worker, so a worker can keep a scratch buffer
/// alive across the roots it processes instead of allocating one per root.
pub(super) fn run<T, C, P>(
    images: usize,
    jobs: usize,
    context: impl Fn() -> C + Sync,
    produce: P,
    sink: &mut impl FnMut(T) -> Result<(), QueryError>,
) -> Result<(), QueryError>
where
    T: Send,
    C: Send,
    P: Fn(&mut C, usize, &mut dyn FnMut(T) -> Result<(), QueryError>) -> Result<(), QueryError>
        + Sync,
{
    let workers = jobs.min(images.max(1));
    let (sender, receiver) = sync_channel::<Message<T>>(workers);
    let next = AtomicUsize::new(0);
    thread::scope(|scope| {
        for _ in 0..workers {
            let sender = sender.clone();
            let produce = &produce;
            let context = &context;
            let next = &next;
            scope.spawn(move || {
                let mut context = context();
                worker_loop(next, images, &mut context, produce, &sender);
            });
        }
        drop(sender);
        consume(receiver, sink)
    })
}

fn worker_loop<T, C, P>(
    next: &AtomicUsize,
    images: usize,
    context: &mut C,
    produce: &P,
    sender: &SyncSender<Message<T>>,
) where
    T: Send,
    P: Fn(&mut C, usize, &mut dyn FnMut(T) -> Result<(), QueryError>) -> Result<(), QueryError>
        + Sync,
{
    loop {
        // Claim whole roots dynamically: a slow image must not leave the other
        // workers idle behind a fixed stride assignment.
        let ordinal = next.fetch_add(1, Ordering::Relaxed);
        if ordinal >= images {
            return;
        }
        let mut emit = |row| {
            sender
                .send(Message::Row(row))
                .map_err(|_| QueryError::WorkerLost)
        };
        if let Err(error) = produce(context, ordinal, &mut emit) {
            let _ = sender.send(Message::Failed(error));
            return;
        }
    }
}

fn consume<T>(
    receiver: Receiver<Message<T>>,
    sink: &mut impl FnMut(T) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    while let Ok(message) = receiver.recv() {
        match message {
            Message::Row(row) => sink(row)?,
            Message::Failed(error) => return Err(error),
        }
    }
    Ok(())
}
