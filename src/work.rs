use crate::errors::WorkError;
use futures::future::{result, Future};
use rusoto_sqs::Message as SqsMessage;

use crate::errors::ProcessorError;
use serde_json;
use std::convert::From;
use std::str::FromStr;

type WorkerFuture = Future<Item = (), Error = WorkError> + Send;

pub trait Worker {
    fn process(&self, message: SqsMessage) -> Box<WorkerFuture>;
}

#[derive(Clone, Default)]
pub struct WorkerImpl;

impl Worker for WorkerImpl {
    fn process(&self, m: SqsMessage) -> Box<WorkerFuture> {
        let f =
            result(m.body.to_owned().ok_or(WorkError::UnRecoverableError(
                "Message contains no body".to_owned(),
            )))
            .and_then(|body| {
                result(WorkLoad::from_str(body.as_ref()).map_err(|e| {
                    WorkError::UnRecoverableError(format!("Invalid Workload {:?}", e))
                }))
            })
            .map(|workload| {
                println!("Received workload: {:#?}", &workload);
                ()
            });
        Box::new(f)
    }
}

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

#[derive(Debug, Clone)]
pub struct Message {
    pub receipt_handle: Option<String>,
    pub message_id: Option<String>,
    pub work_load: Option<WorkLoad>,
}
