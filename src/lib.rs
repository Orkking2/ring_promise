#![doc(html_root_url = "https://docs.rs/ring_promise/0.1.0")]
//! # ring_promise
//!
//! A minimal, ergonomic, and thread-friendly abstraction for submission/completion ring-based I/O.
//! This crate provides a generic interface for working with submission and completion queues,
//! allowing for flexible and efficient asynchronous I/O patterns.
//!
//! ## Features
//! - Generic over submission and completion queue entry types
//! - Thread-safe sender abstraction
//! - Promise-based completion notification
//! - Minimal dependencies

use std::{
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

/// Represents a signal sent to the ring thread.
///
/// - `Entry(T, S)`: Submits an entry of type `T` with a sender for completion of type `S`.
/// - `Reap`: Requests the ring to reap completions.
#[derive(Debug)]
pub enum Signal<T, S> {
    /// Submit an entry and a completion sender.
    Entry(T, S),
    /// Request to reap completions.
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

pub mod registry;
pub mod traits;

/// A thread-safe sender for submitting entries to a ring and receiving completions as promises.
///
/// `PRingSender` is generic over submission queue entries (`S`) and completion queue entries (`C`).
/// It spawns a background thread that manages the ring and handles submission and completion events.
///
/// This struct is safe to share between threads as it is an abstraction over `std::mpsc::Sender`, so it
/// can be used in exactly the same way as one would use that.
#[derive(Clone)]
pub struct PRingSender<S: SQE, C: CQE> {
    /// The channel sender for communicating with the ring thread.
    sender: Sender<Signal<S, oneshot::Sender<C>>>,
}

impl<S: SQE, C: CQE> PRingSender<S, C> {
    /// Creates a new `PRingSender` from a ring instance.
    ///
    /// # Type Parameters
    /// - `SQ`: The submission queue type.
    /// - `CQ`: The completion queue type.
    /// - `Ring`: The full ring type implementing `FullRing`.
    ///
    /// # Arguments
    /// * `ring` - The ring instance to manage.
    ///
    /// # Returns
    /// A new `PRingSender` instance.
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

    /// Generates the background thread function for managing the ring.
    /// Handles submission, completion, and reaping logic.
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
                    Signal::Entry(mut entry, tx) => {
                        // We need the user data to be trackable.
                        let entry_ud = registry.next_uuid();
                        entry.set_ud(entry_ud);

                        // Submit to the registry.
                        registry.insert(entry_ud, tx);

                        // Temporary holder for the entry, required by rust's ownership shinnanigans.
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
                        // Might as well reap the CQ as well.
                        reap(&mut ring, &mut registry);
                    }
                    Signal::Reap => {
                        reap(&mut ring, &mut registry);
                    }
                }
            }
            // Thread joins when receiver produces a `None`, which happens when the last sender (PRingSender) gets dropped.
            // This means we don't actually have to keep track of this thread at all, it will take care of itself.
        }
    }

    /// Creates a new promise and a oneshot sender for completion.
    #[inline]
    fn new_promise(&self) -> (Promise<C, RecvError>, oneshot::Sender<C>) {
        let (tx, rx) = oneshot::channel();

        (Promise::new(move || rx.recv()), tx)
    }

    /// Sends a signal to the ring thread.
    #[inline]
    pub fn send(&self, signal: Signal<S, oneshot::Sender<C>>) {
        self.sender.send(signal).unwrap();
    }

    /// Requests the ring thread to reap completions.
    #[inline]
    pub fn reap(&self) {
        self.send(Signal::Reap);
    }

    /// Submits an entry to the ring and returns a promise for its completion.
    ///
    /// # Arguments
    /// * `entry` - The submission queue entry to submit.
    ///
    /// # Returns
    /// A `Promise` that resolves to the completion queue entry or a receive error.
    #[inline]
    pub fn submit(&self, entry: S) -> Promise<C, RecvError> {
        let (promise, tx) = self.new_promise();

        self.send(Signal::Entry(entry, tx));

        promise
    }

    /// Submits a batch of entries to the ring, returning a vector of promises for their completions.
    ///
    /// # Arguments
    /// * `entries` - An iterator of submission queue entries.
    ///
    /// # Returns
    /// A vector of `Promise`s, one for each entry submitted.
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
