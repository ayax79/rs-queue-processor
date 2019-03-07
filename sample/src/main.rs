#![feature(await_macro, async_await, futures_api)]

#[macro_use]
extern crate serde_derive;

use env_logger;
use futures::future::{result, Future};
use rs_queue_processor::config::Cli;
use rs_queue_processor::errors::WorkError;
use rs_queue_processor::processor::Processor;
use rs_queue_processor::work::Worker;
use rusoto_sqs::Message as SqsMessage;
use std::str::FromStr;

fn main() {
    env_logger::init();

    match Cli::new().build_config() {
        Ok(config) => {
            let worker = WorkerImpl::default();
            let processor = Processor::new(&config, Box::new(worker)).unwrap();
            tokio::run_async(
                async move {
                    let f = processor.process();
                    await!(f).unwrap();
                },
            );
        }
        Err(e) => {
            panic!("{}", e);
        }
    }
}

#[derive(Clone, Default)]
pub struct WorkerImpl;

impl Worker for WorkerImpl {
    fn process(&self, m: SqsMessage) -> Result<(), WorkError> {
        m.body
            .to_owned()
            .ok_or(WorkError::UnRecoverableError(
                "Message contains no body".to_owned(),
            ))
            .and_then(|body| {
                WorkLoad::from_str(body.as_ref())
                    .map_err(|e| WorkError::UnRecoverableError(format!("Invalid Workload {:?}", e)))
            })
            .map(|workload| {
                println!("Received workload: {:#?}", &workload);
                ()
            })
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
