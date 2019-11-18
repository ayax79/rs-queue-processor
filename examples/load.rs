#![feature(async_await, futures_api)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

use chrono::{DateTime, Utc};
use env_logger;
use rs_queue_processor::config::{Cli, Mode};
use rs_queue_processor::errors::WorkError;
use rs_queue_processor::processor::Processor;
use rs_queue_processor::work::Worker;
use rusoto_core::Region;
use rusoto_sqs::Message as SqsMessage;
use rusoto_sqs::{SendMessageError, SendMessageRequest, Sqs, SqsClient as RusotoSqsClient};
use std::default::Default;
use std::str::FromStr;
use std::sync::Arc;
use tokio_async_await::compat::forward::IntoAwaitable;
use tokio_executor::enter;
use uuid::Uuid;

fn main() {
    env_logger::init();
    let config = Cli::new().build_config().unwrap();
    let worker = WorkerImpl::default();
    let (sqs_client, queue_url) = if let Mode::Local(port, queue_url) = &config.mode {
        (build_sqs_client(port.to_owned()), queue_url.clone())
    } else {
        panic!("Could not create an sqs client");
    };

    tokio::run_async(
        async move {
            let mut processor = Processor::new(&config, Box::new(worker)).unwrap();
            processor.start();

            loop {
                if let Err(e) = end_message(
                    Arc::clone(&sqs_client),
                    queue_url.clone(),
                    WorkLoad::default()
                ).await {
                    eprintln!("Error sending message: {}", e)
                }
            }
        },
    );
}

#[derive(Clone, Default)]
pub struct WorkerImpl;

impl Worker for WorkerImpl {
    fn process(&self, m: SqsMessage) -> Result<(), WorkError> {
        let workload = parse_workload(&m)?;
        let difference = millis_since_start(&workload.creation);
        println!(
            "Message {} took {} millis to process",
            &workload.message_id, difference
        );
        Ok(())
    }
}

fn millis_since_start(dt: &DateTime<Utc>) -> i64 {
    Utc::now().timestamp_millis() - dt.timestamp_millis()
}

fn parse_workload(m: &SqsMessage) -> Result<WorkLoad, WorkError> {
    m.body
        .to_owned()
        .ok_or(WorkError::UnRecoverableError(
            "Message contains no body".to_owned(),
        ))
        .and_then(|body| {
            WorkLoad::from_str(body.as_ref())
                .map_err(|e| WorkError::UnRecoverableError(format!("Invalid Workload {:?}", e)))
        })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkLoad {
    pub message_id: Uuid,
    pub creation: DateTime<Utc>,
}

impl Default for WorkLoad {
    fn default() -> Self {
        WorkLoad {
            message_id: Uuid::new_v4(),
            creation: Utc::now(),
        }
    }
}

impl FromStr for WorkLoad {
    type Err = WorkError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        serde_json::from_str(s)
            .map_err(|e| WorkError::UnRecoverableError(format!("Json Error occurred: {:#?}", e)))
    }
}

fn build_local_region(port: u32) -> Region {
    Region::Custom {
        name: "local".to_owned(),
        endpoint: format!("http://localhost:{}", port),
    }
}

fn build_sqs_client(port: u32) -> Arc<RusotoSqsClient> {
    Arc::new(RusotoSqsClient::new(build_local_region(port)))
}

async fn send_message(
    client: Arc<RusotoSqsClient>,
    queue_url: String,
    workload: WorkLoad,
) -> Result<(), String> {
    let json = serde_json::to_string(&workload).map_err(|e| {
        eprintln!("Payload cannot be serialized: {:?}", e);
        format!("send_message error: {:?}", e)
    })?;

    println!("queue_url: {}", &queue_url);
    let mut request = SendMessageRequest::default();
    request.queue_url = queue_url.to_owned();
    request.message_body = json;
    client.send_message(request).into_awaitable()
        .await
        .map(|result| {
            debug!("send message result: {:?}", &result);
            ()
        })
        .map_err(|e| match e {
            SendMessageError::Unknown(be) => {
                let message = String::from_utf8_lossy(be.body.as_slice());
                format!("Unknown Error sending SQS message: {:#?}", message)
            }
            _ => format!("Error receiving SQS message: {}", e),
        })
}
