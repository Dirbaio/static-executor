#![feature(type_alias_impl_trait)]
#![feature(const_in_array_repeat_expressions)]

extern crate static_executor_std;

use async_io::Timer;
use static_executor::{run, task};
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
        match unsafe { work.spawn(i) } {
            Ok(_) => println!("worker spawned"),
            Err(e) => println!("failed to spawn worker: {:?}", e),
        }
        i += 1;
        Timer::after(Duration::from_secs(1)).await;
    }
}

fn main() {
    unsafe {
        tick.spawn().unwrap();
        run()
    }
}
