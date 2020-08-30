#![no_std]

#[no_mangle]
pub extern "C" fn static_executor_signal() {
    cortex_m::asm::sev();
}

#[no_mangle]
pub extern "C" fn static_executor_wait() {
    cortex_m::asm::wfe();
}
