use crate::errors::ProcessorError;
use serde_json;
use std::convert::From;
use std::str::FromStr;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkLoad {
    pub text: String,
}

impl FromStr for WorkLoad {
    type Err = ProcessorError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        serde_json::from_str(s).map_err(ProcessorError::from)
    }
}

#[derive(Debug)]
pub struct Message {
    pub message_id: Option<String>,
    pub work_load: Option<WorkLoad>
}

impl Message {
    pub fn new(message_id: Option<String>, work_load: Option<WorkLoad>) -> Self {
        Message {
            message_id,
            work_load
        }
    }
}
