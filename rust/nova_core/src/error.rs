//! Structured error codes and last-error propagation across the FFI boundary.

use std::cell::RefCell;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NovaStatus {
    Ok = 0,
    InvalidArgument = 1,
    NotFound = 2,
    InternalError = 3,
    PanicCaptured = 4,
}

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

pub fn clear_last_error() {
    LAST_ERROR.with(|cell| cell.borrow_mut().clear());
}

pub fn set_last_error(message: impl Into<String>) {
    let msg = message.into();
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = msg;
    });
}

pub fn take_last_error() -> String {
    LAST_ERROR.with(|cell| cell.borrow().clone())
}
