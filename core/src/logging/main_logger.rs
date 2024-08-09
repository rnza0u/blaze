use super::colors::colorize;
use blaze_common::logger::{LogLevel, Logger, LoggingStrategy};
use colored::*;
use rand::{thread_rng, RngCore};
use std::io::Write;

pub fn get_logger(level: LogLevel) -> Logger {
    Logger::new(MainLoggingStrategy::new(level))
}

pub fn get_contextual_logger(level: LogLevel, context: &str) -> Logger {
    let mut strategy = MainLoggingStrategy::new(level);
    strategy.set_context(context);
    Logger::new(strategy)
}

fn fmt_log_level(level: LogLevel) -> ColoredString {
    let raw = level.to_string().to_uppercase();

    match level {
        LogLevel::Info => colorize(raw, ColoredString::bright_green),
        LogLevel::Error => colorize(raw, ColoredString::red),
        LogLevel::Warn => colorize(raw, ColoredString::yellow),
        LogLevel::Debug => colorize(raw, ColoredString::bright_purple),
    }
}

#[derive(Clone)]
struct Context {
    identifier: String,
    color: colored::CustomColor,
}

/// Blaze main logging strategy. Logs directly to stdout/stderr.
#[derive(Default, Clone)]
struct MainLoggingStrategy {
    level: LogLevel,
    context: Option<Context>,
}

impl MainLoggingStrategy {
    fn new(level: LogLevel) -> Self {
        Self {
            level,
            context: None,
        }
    }

    fn set_context(&mut self, context: &str) {
        let mut random = thread_rng();
        let base_color = [68_u8, 213, 252];
        let mut random_color = [0_u8; 3];
        random.fill_bytes(&mut random_color);
        let avg = |random: u8, base: u8, m: u8| {
            (((random as u64) + (base as u64 * m as u64)) / (1_u64 + m as u64)) as u8
        };
        self.context = Some(Context {
            color: colored::CustomColor {
                r: avg(random_color[0], base_color[0], 2),
                g: avg(random_color[1], base_color[1], 2),
                b: avg(random_color[2], base_color[2], 2),
            },
            identifier: context.to_owned(),
        });
    }
}

impl LoggingStrategy for MainLoggingStrategy {
    fn log(&self, message: &str, level: LogLevel) {
        if level < self.level {
            return;
        }

        let write_to_stream = |s: &mut dyn Write| {
            let mut parts = vec![];

            if let Some(context) = &self.context {
                parts.push(
                    colorize(format!("{} | ", context.identifier), |prefix| {
                        prefix.custom_color(context.color)
                    })
                    .to_string(),
                );
            }

            parts.push(format!("[{}] {}\n", fmt_log_level(level), message));

            let _ = s.write_all(parts.concat().as_bytes());
        };

        match level {
            LogLevel::Info => write_to_stream(&mut std::io::stdout()),
            _ => write_to_stream(&mut std::io::stderr()),
        };
    }
}
