use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

/// The consumer stopped accepting rows, so the worker must stop too. Callers
/// convert it into their own classified failure; the consumer's own error is
/// returned unchanged by [`run`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EmitFailed;

/// Workers deliver completed rows in batches: a wide query produces hundreds of
/// thousands of rows, and one channel handoff per row costs more than the scan
/// that found it. Channel closure still propagates sink failure directly.
enum Message<T, E> {
    Batch(Vec<T>),
    Failed(E),
}

/// Rows a worker accumulates before one channel handoff.
const BATCH_ROWS: usize = 256;

/// Runs `produce` over every image on a worker pool.
///
/// `context` is called once per worker, so a worker can keep a scratch buffer
/// alive across the roots it processes instead of allocating one per root. The
/// error type belongs to the caller so one pool serves row scans, image header
/// reads and class-name resolution.
pub(super) fn run<T, C, P, E>(
    images: usize,
    jobs: usize,
    context: impl Fn() -> C + Sync,
    produce: P,
    sink: &mut impl FnMut(T) -> Result<(), E>,
) -> Result<(), E>
where
    T: Send,
    C: Send,
    E: Send + From<EmitFailed>,
    P: Fn(&mut C, usize, &mut dyn FnMut(T) -> Result<(), E>) -> Result<(), E> + Sync,
{
    let workers = jobs.min(images.max(1));
    // One image needs no channel and no thread: a single-DEX request must not pay
    // pool startup. Multi-image runs keep their producer/consumer split, which is
    // what lets formatting overlap the caller's output even with one worker.
    if images <= 1 {
        let mut context = context();
        for ordinal in 0..images {
            let mut emit = |row| sink(row);
            produce(&mut context, ordinal, &mut emit)?;
        }
        return Ok(());
    }
    let (sender, receiver) = sync_channel::<Message<T, E>>(workers);
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

fn worker_loop<T, C, P, E>(
    next: &AtomicUsize,
    images: usize,
    context: &mut C,
    produce: &P,
    sender: &SyncSender<Message<T, E>>,
) where
    T: Send,
    E: Send + From<EmitFailed>,
    P: Fn(&mut C, usize, &mut dyn FnMut(T) -> Result<(), E>) -> Result<(), E> + Sync,
{
    loop {
        // Claim whole roots dynamically: a slow image must not leave the other
        // workers idle behind a fixed stride assignment.
        let ordinal = next.fetch_add(1, Ordering::Relaxed);
        if ordinal >= images {
            return;
        }
        // A closed channel means the consumer already failed; the producer keeps
        // its own error type and stops through the ordinary failure path.
        let mut batch: Vec<T> = Vec::new();
        let mut closed = false;
        let result = {
            let mut emit = |row| {
                batch.push(row);
                if batch.len() >= BATCH_ROWS
                    && sender
                        .send(Message::Batch(std::mem::take(&mut batch)))
                        .is_err()
                {
                    closed = true;
                    return Err(E::from(EmitFailed));
                }
                Ok(())
            };
            produce(context, ordinal, &mut emit)
        };
        if closed {
            return;
        }
        if !batch.is_empty() && sender.send(Message::Batch(batch)).is_err() {
            return;
        }
        if let Err(error) = result {
            let _ = sender.send(Message::Failed(error));
            return;
        }
    }
}

fn consume<T, E>(
    receiver: Receiver<Message<T, E>>,
    sink: &mut impl FnMut(T) -> Result<(), E>,
) -> Result<(), E> {
    while let Ok(message) = receiver.recv() {
        match message {
            Message::Batch(rows) => {
                for row in rows {
                    sink(row)?;
                }
            }
            Message::Failed(error) => return Err(error),
        }
    }
    Ok(())
}
