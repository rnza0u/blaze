pub mod system_time_as_timestamps {

    use paste::paste;
    use serde::{de::Visitor, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u128(
            time.duration_since(UNIX_EPOCH)
                .map_err(|err| {
                    serde::ser::Error::custom(format!(
                        "could not serialize {time:?} as a unix timestamp ({err})"
                    ))
                })?
                .as_millis(),
        )
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        macro_rules! visit_convert {
            ($t:ty) => {
                paste! {
                    fn [<visit_ $t>]<E>(self, v: $t) -> Result<Self::Value, E>
                        where
                            E: serde::de::Error {
                        u64::try_from(v)
                            .map_err(|err| E::custom(err.to_string()))
                    }
                }
            };
        }

        macro_rules! visit_cast {
            ($t:ty) => {
                paste! {
                    fn [<visit_ $t>]<E>(self, v: $t) -> Result<Self::Value, E>
                        where
                            E: serde::de::Error {
                        Ok(v as u64)
                    }
                }
            };
        }

        struct U64Visitor;

        impl<'de> Visitor<'de> for U64Visitor {
            type Value = u64;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number of milliseconds representing a unix timestamp")
            }

            visit_cast!(u8);
            visit_cast!(u16);
            visit_cast!(u32);

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }

            visit_convert!(u128);

            visit_convert!(i8);
            visit_convert!(i16);
            visit_convert!(i32);
            visit_convert!(i64);
            visit_convert!(i128);
        }

        let millis = deserializer.deserialize_u64(U64Visitor)?;
        Ok(UNIX_EPOCH + Duration::from_millis(millis))
    }
}
