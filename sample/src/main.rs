#[macro_use]
extern crate serde_derive;

extern crate log;

use env_logger;
use futures::future::{result, Future};
use rs_queue_processor::config::Cli;
use rs_queue_processor::errors::WorkError;
use rs_queue_processor::processor::Processor;
use rs_queue_processor::work::{Worker, WorkerFuture};
use rusoto_sqs::Message as SqsMessage;
use std::str::FromStr;

fn main() {
    env_logger::init();

    match Cli::new().build_config() {
        Ok(config) => {
            let worker = WorkerImpl::default();
            let processor = Processor::new(&config, Box::new(worker));
            tokio::run(processor.unwrap().process());
        }
        Err(e) => {
            panic!("{}", e);
        }
    }
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
    type Err = WorkError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        serde_json::from_str(s)
            .map_err(|e| WorkError::UnRecoverableError(format!("Json Error occurred: {:#?}", e)))
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub receipt_handle: Option<String>,
    pub message_id: Option<String>,
    pub work_load: Option<WorkLoad>,
}
