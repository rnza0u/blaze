use blaze_common::shell::Shell;
use serde::Deserialize;

pub enum UseShell {
    Custom(Shell),
    SystemDefault,
}

impl<'de> Deserialize<'de> for UseShell {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum UseShellDeserializationModes {
            True(bool),
            Custom(Shell),
        }

        Ok(
            match UseShellDeserializationModes::deserialize(deserializer)? {
                UseShellDeserializationModes::Custom(shell) => UseShell::Custom(shell),
                UseShellDeserializationModes::True(true) => UseShell::SystemDefault,
                UseShellDeserializationModes::True(false) => {
                    return Err(serde::de::Error::custom(
                        "false is not a valid shell option",
                    ))
                }
            },
        )
    }
}
