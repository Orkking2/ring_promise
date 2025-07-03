use crate::traits::CompletionQueueEntry as CQE;
use std::collections::HashMap;

pub struct Registry<C: CQE> {
    senders: HashMap<u64, oneshot::Sender<C>>,
    curr_ud: u64,
}

impl<C: CQE> Registry<C> {
    pub fn new() -> Self {
        Self {
            senders: HashMap::new(),
            curr_ud: 0,
        }
    }

    pub fn curr_ud(&self) -> u64 {
        self.curr_ud
    }

    fn incr_ud(&mut self) -> u64 {
        let out = self.curr_ud;
        self.curr_ud = self.curr_ud.wrapping_add(1);
        out
    }

    pub fn next_uuid(&mut self) -> u64 {
        loop {
            let id = self.incr_ud();
            if !self.senders.contains_key(&id) {
                break id;
            }
        }
    }

    pub fn insert(&mut self, user_data: u64, sender: oneshot::Sender<C>) {
        self.senders.insert(user_data, sender);
    }

    pub fn complete(&mut self, entry: C) {
        self.senders
            .remove(&entry.get_ud())
            .map(|sender| sender.send(entry));
    }

    pub fn batch_complete<I>(&mut self, entries: I)
    where
        I: Iterator<Item = C>,
    {
        entries.for_each(|entry| self.complete(entry));
    }
}
