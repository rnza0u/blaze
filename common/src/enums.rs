#[macro_export]
macro_rules! unit_enum_from_str {
    ($iterable_type:ty) => {
        impl std::str::FromStr for $iterable_type {
            type Err = $crate::error::Error;

            fn from_str(value: &str) -> $crate::error::Result<$iterable_type> {
                let lowercase_value = value.to_ascii_lowercase();
                <$iterable_type as strum::IntoEnumIterator>::iter()
                    .find(|t| t.to_string().to_ascii_lowercase() == lowercase_value)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "could not convert \"{}\" to a valid {}, possible values are [{}].",
                            value,
                            stringify!($iterable_type),
                            <$iterable_type as strum::IntoEnumIterator>::iter()
                                .map(|t| t.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
            }
        }
    };
}

#[macro_export]
macro_rules! unit_enum_deserialize {
    ($iterable_type:ty) => {
        impl <'de> serde::de::Deserialize<'de> for $iterable_type {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de> {

                $crate::paste::paste! {

                    struct [<$iterable_type Visitor>];

                    impl serde::de::Visitor<'_> for [<$iterable_type Visitor>] {

                        type Value = $iterable_type;

                        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                            formatter.write_str(&format!(
                                "one of: {}",
                                <$iterable_type as strum::IntoEnumIterator>::iter()
                                    .map(|variant| variant.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ))
                        }

                        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
                            where
                                E: serde::de::Error, {
                            <$iterable_type as std::str::FromStr>::from_str(v).map_err(|from_str_err| {
                                E::custom(from_str_err.to_string())
                            })
                        }
                    }

                    deserializer.deserialize_str([<$iterable_type Visitor>])
                }
            }
        }
    }
}

pub use {unit_enum_deserialize, unit_enum_from_str};
