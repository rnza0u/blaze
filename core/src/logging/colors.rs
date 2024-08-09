use colored::ColoredString;

use crate::system::env::Env;

const COLORS_ENV_VARIABLE: &str = "BLAZE_COLORS";

pub fn colorize<S: AsRef<str>, F: FnOnce(ColoredString) -> ColoredString>(
    source: S,
    colorizer: F,
) -> ColoredString {
    if let Some(false) = Env::get_and_deserialize::<bool>(COLORS_ENV_VARIABLE)
        .ok()
        .flatten()
    {
        ColoredString::from(source.as_ref())
    } else {
        colorizer(ColoredString::from(source.as_ref()))
    }
}
