use crate::errors::ProcessorError;
use serde_json;
use std::convert::From;
use std::str::FromStr;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OurMessage {
    pub text: String,
}

impl FromStr for OurMessage {
    type Err = ProcessorError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        serde_json::from_str(s).map_err(ProcessorError::from)
    }
}
