use std::{
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

pub enum Signal<T, S> {
    Entry(T, S),
    Reap,
}

use oneshot::RecvError;
use promisery::Promise;

use crate::{
    registry::Registry,
    traits::{
        CompletionQueue, CompletionQueueEntry as CQE, FullRing, SubmissionQueue,
        SubmissionQueueEntry as SQE,
    },
};

pub mod error;
pub mod registry;
pub mod traits;

#[derive(Clone)]
pub struct PRingSender<S: SQE, C: CQE> {
    sender: Sender<Signal<S, oneshot::Sender<C>>>,
}

impl<S: SQE, C: CQE> PRingSender<S, C> {
    pub fn new<SQ, CQ, Ring>(ring: Ring) -> Self
    where
        SQ: SubmissionQueue<S> + 'static,
        CQ: CompletionQueue<C> + 'static,
        Ring: FullRing<S, C, SQ, CQ> + Send + 'static,
    {
        let (sender, receiver) = channel();

        thread::spawn(Self::thread_fn_generator(ring, receiver));

        Self { sender }
    }

    fn thread_fn_generator<Ring, SQ, CQ>(
        mut ring: Ring,
        receiver: Receiver<Signal<S, oneshot::Sender<C>>>,
    ) -> impl FnOnce() -> ()
    where
        SQ: SubmissionQueue<S>,
        CQ: CompletionQueue<C>,
        Ring: FullRing<S, C, SQ, CQ> + 'static,
    {
        move || {
            let mut registry = Registry::new();

            let reap = |ring: &mut Ring, registry: &mut Registry<C>| {
                registry.batch_complete(ring.completion());
            };

            // Blocks when there are no `Signal`s to consume. Returns `None` when every sender has been dropped.
            for signal in receiver {
                match signal {
                    Signal::Entry(entry, tx) => {
                        let entry_ud = entry.get_ud();

                        registry.insert(entry_ud, tx);

                        let mut entry_holder = Some(entry);

                        // Loops until submission of entry is successful.
                        // Fails if the SQ is full, possible if we are handed a ring with a full SQ or
                        // we have been pushing SQEs and not reaping their CQEs.
                        while let Err(failure_entry) =
                            ring.submission().push(entry_holder.take().unwrap())
                        {
                            entry_holder = Some(failure_entry);

                            // The SQ could be full because the CQ is full.
                            reap(&mut ring, &mut registry);
                            // CQ is now empty, so we should wake the kernel.
                            ring.submit();
                        }

                        // Inform the kernel of our new submission.
                        ring.submit();
                    }
                    Signal::Reap => {
                        reap(&mut ring, &mut registry);
                    }
                }
            }
            // Thread joins when receiver produces a `None`, which happens when the last sender (PIoUring) gets dropped.
            // This means we don't actually have to keep track of this thread at all, it will take care of itself.
        }
    }

    #[inline]
    fn new_promise(&self) -> (Promise<C, RecvError>, oneshot::Sender<C>) {
        let (tx, rx) = oneshot::channel();

        (Promise::new(move || rx.recv()), tx)
    }

    #[inline]
    pub fn send(&self, signal: Signal<S, oneshot::Sender<C>>) {
        self.sender.send(signal).unwrap();
    }

    #[inline]
    pub fn reap(&self) {
        self.send(Signal::Reap);
    }

    #[inline]
    pub fn submit(&self, entry: S) -> Promise<C, RecvError> {
        let (promise, tx) = self.new_promise();

        self.send(Signal::Entry(entry, tx));

        promise
    }

    #[inline]
    pub fn batch_submit<I>(&self, entries: I) -> Vec<Promise<C, RecvError>>
    where
        I: IntoIterator<Item = S>,
    {
        entries
            .into_iter()
            .map(|entry| self.submit(entry))
            .collect::<Vec<_>>()
    }
}
