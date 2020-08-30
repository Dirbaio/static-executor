#![no_std]
#![feature(const_generics)]
#![feature(type_alias_impl_trait)]
#![feature(const_in_array_repeat_expressions)]
#![feature(const_fn)]

use core::future::Future;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

//=============
// Data structures

const STATE_RUNNING: u32 = 1 << 0;
const STATE_QUEUED: u32 = 1 << 1;

struct Header {
    state: AtomicU32,
    next: AtomicPtr<Header>,
    poll_fn: Option<unsafe fn(*mut Header) -> bool>,
}
#[repr(C)]
struct Task<F: Future + 'static> {
    header: Header,
    future: MaybeUninit<F>,
}

//=============
// Atomic task queue using a very, very simple lock-free linked-list queue:
//
// To enqueue a task, task.next is set to the old head, and head is atomically set to task.
//
// Dequeuing is done in batches: the queue is emptied by atomically replacing head with
// null. Then the batch is iterated following the next pointers until null is reached.
//
// Note that batches will be iterated in the opposite order as they were enqueued. This should
// be OK for our use case. Hopefully it doesn't create executor fairness problems.

static QUEUE_HEAD: AtomicPtr<Header> = AtomicPtr::new(ptr::null_mut());

fn enqueue(item: *mut Header) {
    let mut prev = QUEUE_HEAD.load(Ordering::Acquire);
    loop {
        unsafe { (*item).next.store(prev, Ordering::Relaxed) };
        match QUEUE_HEAD.compare_exchange_weak(prev, item, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(next_prev) => prev = next_prev,
        }
    }
}

fn process_queue(on_task: impl Fn(*mut Header)) {
    loop {
        let mut task = QUEUE_HEAD.swap(ptr::null_mut(), Ordering::AcqRel);

        if task.is_null() {
            return;
        }

        while !task.is_null() {
            on_task(task);
            task = unsafe { (*task).next.load(Ordering::Relaxed) };
        }
    }
}

//=============
// Waker

static WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(waker_clone, waker_wake, waker_wake, waker_drop);

unsafe fn waker_clone(p: *const ()) -> RawWaker {
    RawWaker::new(p, &WAKER_VTABLE)
}

unsafe fn waker_wake(p: *const ()) {
    let header = &*(p as *const Header);

    let mut current = header.state.load(Ordering::Acquire);
    loop {
        // If already scheduled, or if not started,
        if (current & STATE_QUEUED != 0) || (current & STATE_RUNNING == 0) {
            return;
        }

        // Mark it as scheduled
        let new = current | STATE_QUEUED;

        match header
            .state
            .compare_exchange_weak(current, new, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => break,
            Err(next_current) => current = next_current,
        }
    }

    // We have just marked the task as scheduled, so enqueue it.
    enqueue(p as *mut Header);

    // TODO: signal the executor the queue is no longer empty.
    // - bare metal: SEV
    // - RTOS: give a semaphore
}

unsafe fn waker_drop(_: *const ()) {
    // nop
}

//=============
// Executor

unsafe fn poll<F: Future + 'static>(p: *mut Header) -> bool {
    let this = &mut *(p as *mut Task<F>);
    let future = Pin::new_unchecked(&mut *this.future.as_mut_ptr());
    let waker = Waker::from_raw(RawWaker::new(p as _, &WAKER_VTABLE));
    let mut cx = Context::from_waker(&waker);
    match future.poll(&mut cx) {
        Poll::Ready(_) => true,
        Poll::Pending => false,
    }
}

pub fn run() {
    loop {
        process_queue(|p| unsafe {
            let header = &*p;

            let state = header.state.fetch_and(!STATE_QUEUED, Ordering::AcqRel);
            if state & STATE_RUNNING == 0 {
                // If task is not running, ignore it. This can happen in the following scenario:
                //   - Task gets dequeued, poll starts
                //   - While task is being polled, it gets woken. It gets placed in the queue.
                //   - Task poll finishes, returning done=true
                //   - RUNNING bit is cleared, but the task is already in the queue.
                return;
            }

            // Unchecked unwrap. run_fn must be set because it's set when starting
            // the task.
            let poll_fn = header.poll_fn.unwrap_or_else(|| core::hint::unreachable_unchecked());

            // Run the task
            let done = poll_fn(p as _);

            if done {
                header.state.fetch_and(!STATE_RUNNING, Ordering::AcqRel);
            }
        })

        // TODO: if we're here, the queue is empty. In order to not spin the CPU:
        // - for bare metal: WFE
        // - for RTOS: take a semaphore
    }
}

//=============
// Task pool

pub struct TaskPool<F: Future + 'static, const LEN: usize> {
    tasks: [Task<F>; LEN],
}

impl<F: Future + 'static, const LEN: usize> TaskPool<F, LEN> {
    pub const fn new() -> Self {
        Self {
            tasks: [Task {
                header: Header {
                    state: AtomicU32::new(0),
                    next: AtomicPtr::new(ptr::null_mut()),
                    poll_fn: None,
                },
                future: MaybeUninit::uninit(),
            }; LEN],
        }
    }

    unsafe fn allocate(&self) -> Option<&mut Task<F>> {
        for t in &self.tasks[..] {
            let state = STATE_RUNNING | STATE_QUEUED;
            if t.header.state.compare_and_swap(0, state, Ordering::AcqRel) == 0 {
                return Some(&mut *(t as *const Task<F> as *mut Task<F>));
            }
        }
        None
    }

    pub unsafe fn spawn(&'static self, future: F) {
        let task = self.allocate().unwrap();
        task.header.poll_fn = Some(poll::<F>);
        ptr::write(task.future.as_mut_ptr(), future);
        enqueue(&mut task.header as _);
    }
}

#[macro_export]
macro_rules! task_pool {
    ($len:expr, async fn $name:ident ( $($argname:ident: $argtype:ty),*) $block:block) => {

        mod $name {
            type FutureType = impl ::core::future::Future;

            async fn task($($argname: $argtype),*) {
                $block
            }
            fn create($($argname: $argtype),*) -> FutureType {
                task($($argname),*)
            }

            static TASK_POOL: $crate::TaskPool<FutureType, $len> = $crate::TaskPool::new();

            pub fn spawn($($argname: $argtype),*)  {
                unsafe { TASK_POOL.spawn(create($($argname),*)) }
            }
        }
    };
}

#[macro_export]
macro_rules! task {
    (async fn $name:ident ( $($argname:ident: $argtype:ty),*) $block:block) => {
        task_pool!(1, async fn $name($($argname: $argtype),*) $block );
    };
}
