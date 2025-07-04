# ring_promise

A minimal, ergonomic, and thread-friendly abstraction for submission/completion ring-based I/O in Rust.

## Features
- Generic over submission and completion queue entry types
- Thread-safe sender abstraction
- Promise-based completion notification
- Minimal dependencies

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
ring_promise = "0.1.0"
```

### Example

```rust
use ring_promise::{PRingSender, traits::*};
// Define your SQE, CQE, and ring types here...

// let ring = ...;
// let sender = PRingSender::<MySQE, MyCQE>::new::<MySQ, MyCQ, MyRing>(ring);
// let promise = sender.submit(my_entry);
// let result = promise.wait().unwrap(); // Look at the promisery crate for details on handling the promises.
```

## Traits

- `SubmissionQueueEntry`: Types that can be submitted to a submission queue.
- `SubmissionQueue`: A queue for submission entries.
- `CompletionQueueEntry`: Types produced by completion queues.
- `CompletionQueue`: A queue for completion entries (iterator).
- `Submitter`: Notifies the kernel/system of new submissions.
- `FullRing`: Combines submission, completion, and submitter.

## License

MIT OR Apache-2.0
