pub trait SubmissionQueueEntry: Send + 'static {
    fn set_ud(&mut self, ud: u64);
    fn get_ud(&self) -> u64;
}

pub trait SubmissionQueue<S: SubmissionQueueEntry> {
    fn push(&mut self, entry: S) -> Result<(), S>;
}

pub trait CompletionQueueEntry: Send + 'static {
    fn get_ud(&self) -> u64;
}

pub trait CompletionQueue<C: CompletionQueueEntry>: Iterator<Item = C> {}

pub trait Submitter {
    fn submit(&mut self);
}

pub trait FullRing<S, C, SQ, CQ>
where
    S: SubmissionQueueEntry,
    C: CompletionQueueEntry,
    SQ: SubmissionQueue<S>,
    CQ: CompletionQueue<C>,
    Self: Submitter,
{
    fn completion(&mut self) -> CQ;
    fn submission(&mut self) -> SQ;
}
