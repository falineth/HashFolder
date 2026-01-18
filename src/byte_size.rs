use clap::builder::TypedValueParser;
use clap::error::{ContextKind, ContextValue};
use clap::{Arg, Command};

#[derive(Clone, Copy, Debug)]
pub enum ByteSize {
    Byte(u64),
    KByte(u64),
    KiByte(u64),
    MByte(u64),
    MiByte(u64),
    GByte(u64),
    GiByte(u64),
    TByte(u64),
    TiByte(u64),
}

impl Into<u64> for ByteSize {
    fn into(self) -> u64 {
        match self {
            ByteSize::Byte(value) => value,
            ByteSize::KByte(value) => value * 1000,
            ByteSize::KiByte(value) => value * 1024,
            ByteSize::MByte(value) => value * 1_000_000,
            ByteSize::MiByte(value) => value * 1_048_576,
            ByteSize::GByte(value) => value * 1_000_000_000,
            ByteSize::GiByte(value) => value * 1_073_741_824,
            ByteSize::TByte(value) => value * 1_000_000_000_000,
            ByteSize::TiByte(value) => value * 1_099_511_627_776,
        }
    }
}

#[derive(Clone)]
pub struct ByteSizeValueParser {}

impl ByteSizeValueParser {
    /// Parse non-empty string values
    pub fn new() -> Self {
        Self {}
    }
}

impl TypedValueParser for ByteSizeValueParser {
    type Value = ByteSize;

    fn parse_ref(
        &self,
        cmd: &Command,
        arg: Option<&Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::error::Error> {
        let value = value.to_owned().into_string();

        let value = match value {
            Ok(value) => value,
            Err(_) => {
                let mut err =
                    clap::Error::new(clap::error::ErrorKind::ValueValidation).with_cmd(cmd);
                if let Some(arg) = arg {
                    err.insert(
                        ContextKind::InvalidArg,
                        ContextValue::String(arg.to_string()),
                    );
                }

                err.insert(
                    ContextKind::InvalidValue,
                    ContextValue::String(format!("Invalid UTF8")),
                );

                return Err(err);
            }
        };

        if value.bytes().all(|c| c.is_ascii_digit()) {
            if let Ok(value) = value.parse::<u64>() {
                return Ok(ByteSize::Byte(value));
            }
        }

        let suffixes = [
            "B", "KB", "K", "KiB", "MB", "M", "MiB", "GB", "G", "GiB", "TB", "T", "TiB",
        ];

        let valid_byte_size = suffixes
            .iter()
            .filter_map(|suffix| {
                if let Some(value) = value.strip_suffix(suffix) {
                    if value.bytes().all(|c| c.is_ascii_digit()) {
                        Some((value, *suffix))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .next();

        if let Some((value, suffix)) = valid_byte_size {
            if let Ok(value) = value.parse::<u64>() {
                if let Some(result) = match suffix {
                    "B" => Some(ByteSize::Byte(value)),
                    "KB" | "K" => Some(ByteSize::KByte(value)),
                    "KiB" => Some(ByteSize::KiByte(value)),
                    "MB" | "M" => Some(ByteSize::MByte(value)),
                    "MiB" => Some(ByteSize::MiByte(value)),
                    "GB" | "G" => Some(ByteSize::GByte(value)),
                    "GiB" => Some(ByteSize::GiByte(value)),
                    "TB" | "T" => Some(ByteSize::TByte(value)),
                    "TiB" => Some(ByteSize::TiByte(value)),
                    _ => None,
                } {
                    return Ok(result);
                }
            }
        }

        let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation).with_cmd(cmd);
        if let Some(arg) = arg {
            err.insert(
                ContextKind::InvalidArg,
                ContextValue::String(arg.to_string()),
            );
        }

        err.insert(
            ContextKind::InvalidValue,
            ContextValue::String(format!(
                "Unknown \"{value}\", expected [number](KB,KiB,MB,MiB,GB,GiB,TB,TiB)"
            )),
        );

        return Err(err);
    }
}
