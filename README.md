# READ THIS

This repo is now unmaintained.

The executor design lives on as part of [embassy](https://github.com/akiles/embassy). The version there has many fixes and improvements not present in this repo.

# static-executor

Proof of concept of a Rust async executor that statically allocates tasks. Ideal for embedded devices, works
without `std` or `alloc`.

See examples/simple.rs for usage example.

To-Do list:

- safety hole: add a check so that run() cannot be invoked from several threads concurrently.
- Return "pool full" errors from spawn (it currently panics)
- The macros are not very ergonomic, maybe attribute macros would be nicer: `#[task]` and `#[task_pool(4)]`.

Missing features (still undecided if they're in-scope of this project):

- Add JoinHandles. Allows awaiting one task from another.
- Add ability for JoinHandle to retrieve the future's result.
- Add ability for JoinHandle to cancel running tasks.

Non-features:

- Dynamic allocation of tasks. This is fundamentally incompatible with how wakers are implemented.
