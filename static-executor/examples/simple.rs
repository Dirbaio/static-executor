#![feature(type_alias_impl_trait)]
#![feature(const_in_array_repeat_expressions)]

use async_io::Timer;
use static_executor::{task, Executor};
use std::time::Duration;

#[task(pool_size = 4)]
async fn work(secs: u64) {
    println!("work started {}", secs);
    Timer::after(std::time::Duration::from_secs(secs)).await;
    println!("work finished {}", secs);
}

#[task]
async fn tick() {
    let mut i = 1;
    loop {
        println!("tick! {}", i);
        match unsafe { EXECUTOR.spawn(work(i)) } {
            Ok(_) => println!("worker spawned"),
            Err(e) => println!("failed to spawn worker: {:?}", e),
        }
        i += 1;
        Timer::after(Duration::from_secs(1)).await;
    }
}

static EXECUTOR: Executor = Executor::new(executor_signal);
use std::sync::{Condvar, Mutex};

lazy_static::lazy_static! {
    static ref MUTEX: Mutex<bool> = Mutex::new(false);
    static ref CONDVAR: Condvar = Condvar::new();
}

fn executor_signal() {
    let mut signaled = MUTEX.lock().unwrap();
    *signaled = true;
    CONDVAR.notify_one();
}

fn executor_wait() {
    let mut signaled = MUTEX.lock().unwrap();
    while !*signaled {
        signaled = CONDVAR.wait(signaled).unwrap();
    }
    *signaled = false
}

fn main() {
    unsafe {
        EXECUTOR.spawn(tick()).unwrap();
        loop {
            EXECUTOR.run();
            executor_wait();
        }
    }
}
