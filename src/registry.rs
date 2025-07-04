//! Registry for associating user data with completion senders.
//!
//! The `Registry` manages the mapping between user data (u64) and oneshot senders for completion queue entries.
//! It is used internally by the ring thread to track outstanding submissions and deliver completions.

use crate::traits::CompletionQueueEntry as CQE;
use std::collections::HashMap;

/// A registry mapping user data to completion senders.
pub struct Registry<C: CQE> {
    /// Map from user data to oneshot senders.
    senders: HashMap<u64, oneshot::Sender<C>>,
    /// The current user data counter.
    curr_ud: u64,
}

impl<C: CQE> Registry<C> {
    /// Create a new, empty registry.
    pub fn new() -> Self {
        Self {
            senders: HashMap::new(),
            curr_ud: 0,
        }
    }

    /// Get the current user data value.
    pub fn curr_ud(&self) -> u64 {
        self.curr_ud
    }

    /// Return the current user data value, incrementing the internal user data value, wrapping on overflow.
    fn incr_ud(&mut self) -> u64 {
        let out = self.curr_ud;
        self.curr_ud = self.curr_ud.wrapping_add(1);
        out
    }

    /// Get the next unused user data value.
    pub fn next_uuid(&mut self) -> u64 {
        loop {
            let id = self.incr_ud();
            if !self.senders.contains_key(&id) {
                break id;
            }
        }
    }

    /// Insert a sender for a given user data value.
    pub fn insert(&mut self, user_data: u64, sender: oneshot::Sender<C>) {
        self.senders.insert(user_data, sender);
    }

    /// Complete an entry, sending it to the registered sender if present.
    pub fn complete(&mut self, entry: C) {
        self.senders
            .remove(&entry.get_ud())
            // If there is no sender with this user data value, simply ignore it.
            .map(|sender| sender.send(entry));
    }

    /// Complete a batch of entries, sending each to its registered sender.
    pub fn batch_complete<I>(&mut self, entries: I)
    where
        I: Iterator<Item = C>,
    {
        entries.for_each(|entry| self.complete(entry));
    }
}
