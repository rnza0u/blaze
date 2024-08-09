use std::time::SystemTime;

#[cfg(feature = "testing")]
use {
    super::thread::{ThreadScopeId, SCOPE_ID},
    once_cell::sync::Lazy,
    std::{
        collections::HashMap,
        sync::Mutex,
        time::{Duration, UNIX_EPOCH},
    },
};

#[cfg(feature = "testing")]
pub static TEST_TIMES: Lazy<Mutex<HashMap<ThreadScopeId, SystemTime>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[cfg(feature = "testing")]
pub fn set_current_time(time: SystemTime) {
    let _ = TEST_TIMES.lock().unwrap().insert(SCOPE_ID.get(), time);
}

#[cfg(feature = "testing")]
pub fn now() -> SystemTime {
    let scope_id = SCOPE_ID.get();
    let mut times = TEST_TIMES.lock().unwrap();

    let current = times.get(&scope_id).unwrap_or(&UNIX_EPOCH).to_owned();

    let _ = times.insert(scope_id, current + Duration::from_nanos(1));

    current
}

#[cfg(not(feature = "testing"))]
pub fn now() -> SystemTime {
    SystemTime::now()
}
