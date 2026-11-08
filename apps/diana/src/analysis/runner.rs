use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

/// The consumer stopped accepting rows, so the worker must stop too. Callers
/// convert it into their own classified failure; the consumer's own error is
/// returned unchanged by [`run`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EmitFailed;

/// Each worker can queue one completed row; channel closure propagates sink
/// failure directly to producers.
enum Message<T, E> {
    Row(T),
    Failed(E),
}

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
        let mut emit = |row| {
            sender
                .send(Message::Row(row))
                .map_err(|_| E::from(EmitFailed))
        };
        if let Err(error) = produce(context, ordinal, &mut emit) {
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
            Message::Row(row) => sink(row)?,
            Message::Failed(error) => return Err(error),
        }
    }
    Ok(())
}
