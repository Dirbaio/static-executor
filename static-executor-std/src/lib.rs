use std::sync::{Condvar, Mutex};

lazy_static::lazy_static! {
    static ref MUTEX: Mutex<bool> = Mutex::new(false);
    static ref CONDVAR: Condvar = Condvar::new();
}

#[no_mangle]
pub fn _static_executor_signal() {
    let mut signaled = MUTEX.lock().unwrap();
    *signaled = true;
    CONDVAR.notify_one();
}

#[no_mangle]
pub fn _static_executor_wait() {
    let mut signaled = MUTEX.lock().unwrap();
    while !*signaled {
        signaled = CONDVAR.wait(signaled).unwrap();
    }
    *signaled = false
}
