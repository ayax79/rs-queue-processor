use crate::config::{Config, Mode};
use crate::errors::{ProcessorError, WorkError};
use crate::model::Message;
use crate::sqs::SqsClient;
use crate::work::Worker;
use futures::future::{err, ok};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::executor::DefaultExecutor;
use tokio::executor::Executor;
use tokio::prelude::*;
use tokio::timer::Interval;

type ProcessorFuture = Future<Item = (), Error = ()> + Send;
type ProcessorErrorFuture = Future<Item = (), Error = ProcessorError> + Send;
type ShareableWorker = Worker + Send + Sync;

#[derive(Clone)]
pub struct Processor {
    sqs_client: SqsClient,
    worker: Arc<ShareableWorker>,
}

impl Processor {
    pub fn new(config: &Config, worker: Box<ShareableWorker>) -> Result<Self, ProcessorError> {
        println!("Initializing rs-queue-processor: {:?}", &config.mode);
        let sqs_client = build_sqs_client(&config.mode);
        Ok(Processor {
            sqs_client,
            worker: Arc::from(worker),
        })
    }

    pub fn process(&self) {
        // Clone required for the move in for_each. Cloning SqsClient is cheap as the underlying Rusoto client is embedded in an Arc
        let self_clone = self.clone();
        let task = Interval::new(Instant::now(), Duration::from_secs(2))
            .for_each(move |_| {
                debug!("Timer task is starting");
                let f = self_clone.process_messages();
                let _r = DefaultExecutor::current().spawn(f);
                Ok(())
            })
            .map_err(|e| panic!("interval error; err={:#?}", e));

        tokio::run(task);
    }

    fn process_messages(&self) -> Box<ProcessorFuture> {
        let self_clone = self.clone();
        Box::new(
            self.sqs_client
                .fetch_messages()
                .map(move |messages| {
                    if messages.is_empty() {
                        println!("No messages received for queue")
                    } else {
                        for m in messages {
                            let f = self_clone.process_message(m);
                            let _r = DefaultExecutor::current().spawn(f);
                        }
                    }
                })
                .map_err(|e| panic!("An error occurred: {}", e)),
        )
    }

    fn process_message(&self, m: Message) -> Box<ProcessorFuture> {
        let message = m.clone();
        let delete_clone = m.clone();
        let work_error_clone = m.clone();
        let sqs_client_or_else = self.sqs_client.clone(); // clone for if there was an error processing messages
        let sqs_client_and_then = self.sqs_client.clone(); // clone for handle_delete
        let worker = self.worker.clone();
        Box::new(
            worker
                .process(message)
                .map_err(ProcessorError::from)
                .or_else(|ref pe| {
                    if let ProcessorError::WorkErrorOccurred(we) = pe.clone() {
                        handle_work_error(sqs_client_or_else, we.clone(), work_error_clone)
                    } else {
                        processor_error_future(ProcessorError::Unknown)
                    }
                })
                .and_then(|_f| handle_delete(sqs_client_and_then, delete_clone))
                .map_err(|e| error!("Error occurred: {}", e)),
        )
    }
}

fn processor_error_future(pe: ProcessorError) -> Box<ProcessorErrorFuture> {
    Box::new(err(pe))
}

fn build_sqs_client(mode: &Mode) -> SqsClient {
    match mode {
        Mode::AWS(region, queue) => SqsClient::new(region.to_owned(), queue),
        Mode::Local(port, queue) => SqsClient::local(*port, queue),
    }
}

fn handle_delete(sqs_client: SqsClient, message: Message) -> Box<ProcessorErrorFuture> {
    if let Some(receipt_handle) = message.receipt_handle.clone() {
        Box::new(sqs_client.delete_message(receipt_handle.as_ref()))
    } else {
        error!("No receipt id found for message: {:?}", message);
        Box::new(ok(()))
    }
}

fn handle_work_error(
    sqs_client: SqsClient,
    we: WorkError,
    m: Message,
) -> Box<ProcessorErrorFuture> {
    let delete_clone = m.clone();
    Box::new(match we.clone() {
        WorkError::UnRecoverableError(msg) => {
            error!("No way to recover from error: {} deleting", &msg);
            handle_delete(sqs_client, delete_clone)
        }
        WorkError::RecoverableError(msg) => {
            error!("Recoverable from error: {} deleting", &msg);
            handle_delete(sqs_client, delete_clone)
        }
    })
}
