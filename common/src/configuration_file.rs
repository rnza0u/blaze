use crate::enums::{unit_enum_deserialize, unit_enum_from_str};

use serde::Serialize;
use strum_macros::{Display, EnumIter};

/// Configuration file formats.
#[derive(EnumIter, Default, Display, Hash, PartialEq, Eq, Copy, Clone, Debug, Serialize)]
pub enum ConfigurationFileFormat {
    Json,
    Yaml,
    #[default]
    Jsonnet,
}

unit_enum_from_str!(ConfigurationFileFormat);
unit_enum_deserialize!(ConfigurationFileFormat);
