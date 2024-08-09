use std::{
    panic::{RefUnwindSafe, UnwindSafe},
    sync::Arc,
};

use serde::Serialize;
use strum_macros::{Display, EnumIter};

use crate::enums::{unit_enum_deserialize, unit_enum_from_str};

/// Log levels that can be used with a [`Logger`].
#[derive(
    Default, EnumIter, Display, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Debug, Serialize,
)]
pub enum LogLevel {
    Debug,
    #[default]
    Warn,
    Info,
    Error,
}

unit_enum_from_str!(LogLevel);
unit_enum_deserialize!(LogLevel);

/// Implementation of how a logger logs messages. See [`Logger`] struct.
pub trait LoggingStrategy: Send {
    fn log(&self, message: &str, level: LogLevel);
}

/// A logger object that delegates to a [`LoggingStrategy`]. Handles boxing and wrapping in [`Arc`].
#[derive(Clone)]
pub struct Logger(
    Arc<Box<dyn LoggingStrategy + Send + Sync + UnwindSafe + RefUnwindSafe + 'static>>,
);

impl Logger {
    /// Create a new logger object from the provided [`LoggingStrategy`] implementation.
    pub fn new<T: LoggingStrategy + Send + Sync + UnwindSafe + RefUnwindSafe + 'static>(
        strategy: T,
    ) -> Self {
        Self(Arc::new(Box::new(strategy)))
    }

    /// Log a message with a specific log level.
    pub fn log<M: AsRef<str>>(&self, message: M, level: LogLevel) {
        self.0.log(message.as_ref(), level)
    }

    /// Log an information message.
    pub fn info<M: AsRef<str>>(&self, message: M) {
        self.0.log(message.as_ref(), LogLevel::Info)
    }

    /// Log a warning message.
    pub fn warn<M: AsRef<str>>(&self, message: M) {
        self.0.log(message.as_ref(), LogLevel::Warn)
    }

    /// Log an error message.
    pub fn error<M: AsRef<str>>(&self, message: M) {
        self.0.log(message.as_ref(), LogLevel::Error)
    }

    /// Log a debugging message.
    pub fn debug<M: AsRef<str>>(&self, message: M) {
        self.0.log(message.as_ref(), LogLevel::Debug)
    }
}
