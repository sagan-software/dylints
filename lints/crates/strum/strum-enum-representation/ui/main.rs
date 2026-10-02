use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

enum ManualStatus {
    Ready,
    Busy,
}

impl fmt::Display for ManualStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready => formatter.write_str("ready"),
            Self::Busy => formatter.write_str("busy"),
        }
    }
}

impl FromStr for ManualStatus {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ready" => Ok(Self::Ready),
            "busy" => Ok(Self::Busy),
            _ => Err(()),
        }
    }
}

enum DisplayOnly {
    Ready,
}

impl fmt::Display for DisplayOnly {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ready")
    }
}

#[derive(Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
enum AcronymNames {
    HTTPResponse,
}

#[derive(Serialize, Deserialize, Display, EnumString)]
enum DirectionalNames {
    #[serde(rename(serialize = "outbound", deserialize = "inbound"))]
    #[strum(to_string = "displayed", serialize = "inbound")]
    Ready,
}

#[derive(Serialize, Deserialize, Display, EnumString)]
enum InputNames {
    #[serde(rename(serialize = "ready", deserialize = "inbound"))]
    #[strum(to_string = "ready", serialize = "other")]
    Ready,
}

#[derive(Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
enum AlignedNames {
    InProgress,
    #[serde(rename = "ready", alias = "prepared")]
    #[strum(to_string = "ready", serialize = "prepared")]
    Ready,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SerdeOnly {
    HTTPResponse,
}

fn main() {
    // Keep every fixture type reachable without changing its lint contract.
    let _ = ManualStatus::Ready.to_string();
    let _ = "busy".parse::<ManualStatus>();
    let _ = DisplayOnly::Ready.to_string();
    let _ = AcronymNames::HTTPResponse.to_string();
    let _ = DirectionalNames::Ready.to_string();
    let _ = InputNames::Ready.to_string();
    let _ = AlignedNames::InProgress.to_string();
    let _ = SerdeOnly::HTTPResponse;
}
