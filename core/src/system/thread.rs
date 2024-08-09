#[cfg(feature = "testing")]
use std::{cell::Cell, sync::atomic::AtomicUsize};

#[cfg(not(feature = "testing"))]
macro_rules! thread {
    ($f:expr) => {
        std::thread::spawn($f)
    };
    ($s:expr,$f:expr) => {
        $s.spawn($f)
    };
}

#[cfg(feature = "testing")]
pub type ThreadScopeId = usize;

#[cfg(feature = "testing")]
static THREAD_SCOPE_ID_SEQUENCE: AtomicUsize = AtomicUsize::new(usize::MIN);

#[cfg(feature = "testing")]
thread_local! {
    pub(crate) static SCOPE_ID: Cell<ThreadScopeId> = Cell::new(THREAD_SCOPE_ID_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
}

#[cfg(feature = "testing")]
macro_rules! thread {
    ($f:expr) => {{
        use crate::system::thread::SCOPE_ID;
        let current_scope_id = SCOPE_ID.with(|id| id.get());
        std::thread::spawn(move || {
            SCOPE_ID.set(current_scope_id);
            $f()
        })
    }};
    ($s:expr, $f:expr) => {{
        use crate::system::thread::SCOPE_ID;
        let current_scope_id = SCOPE_ID.with(|id| id.get());
        $s.spawn(move || {
            SCOPE_ID.set(current_scope_id);
            $f()
        })
    }};
}

macro_rules! join {
    ($t:expr) => {
        match $t.join() {
            Ok(r) => r,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    };
}

pub(crate) use {join, thread};
