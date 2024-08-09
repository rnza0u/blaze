use std::{fmt::Display, num::NonZeroUsize, str::FromStr};

use serde::{de::Visitor, Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Default, Copy, Clone)]
pub enum Parallelism {
    #[default]
    None,
    Count(NonZeroUsize),
    All,
    Infinite,
}

impl Serialize for Parallelism {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Count(i) => i.serialize(serializer),
            other => serializer.serialize_str(&other.to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for Parallelism {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ParallelismVisitor;

        impl<'de> Visitor<'de> for ParallelismVisitor {
            type Value = Parallelism;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("either a number or a string")
            }

            fn visit_str<E>(self, v: &str) -> std::prelude::v1::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Parallelism::from_str(v).map_err(serde::de::Error::custom)
            }

            fn visit_u64<E>(self, v: u64) -> std::prelude::v1::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Parallelism::Count(
                    usize::try_from(v)
                        .and_then(NonZeroUsize::try_from)
                        .map_err(serde::de::Error::custom)?,
                ))
            }
        }

        deserializer.deserialize_any(ParallelismVisitor)
    }
}

impl FromStr for Parallelism {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "all" => Ok(Self::All),
            "none" => Ok(Self::None),
            "infinite" => Ok(Self::Infinite),
            number => Ok(Self::Count(NonZeroUsize::from_str(number)?)),
        }
    }
}

impl Display for Parallelism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => f.write_str("All"),
            Self::Infinite => f.write_str("Infinite"),
            Self::Count(i) => f.write_str(&i.to_string()),
            Self::None => f.write_str("None"),
        }
    }
}
