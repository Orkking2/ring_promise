//! Traits for generic submission and completion ring-based I/O.
//!
//! These traits define the core abstractions for submission and completion queues, entries, and rings.
//! They are designed to be flexible and extensible for a variety of I/O backends.

/// A submission queue entry.
///
/// Types implementing this trait can be submitted to a submission queue.
pub trait SubmissionQueueEntry: Send + 'static {
    /// Set the user data field for this entry.
    fn set_ud(&mut self, ud: u64);
    /// Get the user data field for this entry.
    fn get_ud(&self) -> u64;
}

/// A submission queue for entries of type `S`.
pub trait SubmissionQueue<S: SubmissionQueueEntry> {
    /// Attempt to push an entry onto the submission queue.
    ///
    /// Returns `Ok(())` if successful, or `Err(entry)` if the queue is full.
    fn push(&mut self, entry: S) -> Result<(), S>;
}

/// A completion queue entry.
///
/// Types implementing this trait are produced by completion queues.
pub trait CompletionQueueEntry: Send + 'static {
    /// Get the user data field for this entry.
    fn get_ud(&self) -> u64;
}

/// A completion queue for entries of type `C`.
///
/// This trait is auto-implemented for any iterator over `C`.
pub trait CompletionQueue<C: CompletionQueueEntry>: Iterator<Item = C> {}

/// A type that can submit entries to the kernel or underlying system.
pub trait Submitter {
    /// Notify the kernel or system that new entries are ready for processing.
    fn submit(&mut self);
}

/// A full ring abstraction, combining submission and completion queues and a submitter.
///
/// This trait ties together the submission and completion queues and provides access to both.
pub trait FullRing<S, C, SQ, CQ>
where
    S: SubmissionQueueEntry,
    C: CompletionQueueEntry,
    SQ: SubmissionQueue<S>,
    CQ: CompletionQueue<C>,
    Self: Submitter,
{
    /// Get the completion queue.
    fn completion(&mut self) -> CQ;
    /// Get the submission queue.
    fn submission(&mut self) -> SQ;
}
