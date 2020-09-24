# static-executor

Proof of concept of a Rust async executor that statically allocates tasks. Ideal for embedded devices, works
without `std` or `alloc`.

See examples/simple.rs for usage example.

Missing features (still undecided if they're in-scope of this project):

- Add JoinHandles. Allows awaiting one task from another.
- Add ability for JoinHandle to retrieve the future's result.
- Add ability for JoinHandle to cancel running tasks.

Non-features:

- Dynamic allocation of tasks. This is fundamentally incompatible with how wakers are implemented.
