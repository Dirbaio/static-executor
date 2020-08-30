#![feature(type_alias_impl_trait)]

use static_executor::{run, task, task_pool};

task_pool!(
    4,
    async fn hello(secs: u64) {
        use async_io::Timer;

        println!("hello {}", secs);
        Timer::after(std::time::Duration::from_secs(secs)).await;
        println!("bye {}", secs);
    }
);

task!(
    async fn tick() {
        use async_io::Timer;

        loop {
            println!("tick!");
            Timer::after(std::time::Duration::from_secs(1)).await;
        }
    }
);

fn main() {
    hello::spawn(1);
    hello::spawn(2);
    hello::spawn(3);
    hello::spawn(4);
    tick::spawn();
    run()
}
