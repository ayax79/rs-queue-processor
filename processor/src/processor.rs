use crate::config::{Config, Mode};
use crate::errors::{ProcessorError, WorkError};
use crate::sqs::SqsClient;
use crate::work::Worker;
use futures::future::Future as OldFuture;
use rusoto_sqs::Message as SqsMessage;
use std::future::Future as NewFuture;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Interval;
use tokio_async_await::compat::forward::IntoAwaitable;

// todo: make configurable
/// Default requeue delay in seconds
const DEFAULT_REQUEUE_DELAY: i64 = 10;

// type ProcessorFuture = dyn Future<Item = (), Error = ()> + Send;
// type ProcessorErrorFuture = dyn Future<Item = (), Error = ProcessorError> + Send;
type ShareableWorker = dyn Worker + Send + Sync;

/// This is the main class for processing messages from an SQS Queue
///
/// To instantiate an instance of Processor you will need:
/// * A configuration object.
/// * A Worker instance that supports both Send and Sync
#[derive(Clone)]
pub struct Processor {
    sqs_client: SqsClient,
    worker: Arc<ShareableWorker>,
}

impl Processor {
    /// Instantiates a new instance of the process
    pub fn new(config: &Config, worker: Box<ShareableWorker>) -> Result<Self, ProcessorError> {
        println!("Initializing rs-queue-processor: {:?}", &config.mode);
        let sqs_client = build_sqs_client(&config.mode);
        Ok(Processor {
            sqs_client,
            worker: Arc::from(worker),
        })
    }

    /// Generates a Interval Task that can be executed
    ///
    /// let processor = Processor::new(&config, worker);
    /// tokio::run(processor.process());
    pub fn process(&self) -> impl NewFuture<Output = Result<(), ()>> + '_ {
        println!("process called!!");
        // Clone required for the move in for_each. Cloning SqsClient is cheap as the underlying Rusoto client is embedded in an Arc
        let self_clone = self.clone();
        Interval::new(Instant::now(), Duration::from_secs(2))
            .for_each(move |_| {
                println!("Timer task is starting");
                let clone_2 = self_clone.clone();
                let _r = tokio::spawn_async(
                    async move {
                        await!(clone_2.process_messages());
                    },
                );
                Ok(())
            })
            .map_err(|e| panic!("interval error; err={:#?}", e))
            .into_awaitable()
    }

    /// Returns a future that will fetch messages from
    /// SQS to be processed
    async fn process_messages(&self) {
        match await!(self.sqs_client.fetch_messages()) {
            Ok(messages) => {
                for message in messages {
                    let result = await!(self.process_message(message.clone()));
                    if let Err(e) = result {
                        error!("Error processing message: {:?} error: {:?}", &message, &e);
                    }
                }
            }
            Err(err) => error!("Error fetching messages: {:?}", err),
        }
    }

    /// Returns a future that will process one message
    /// The message will be passed to the worker.
    async fn process_message(&self, m: SqsMessage) -> Result<(), ProcessorError> {
        println!("Process message called with: {:?}", &m);
        let message = m.clone();
        let delete_clone = m.clone();
        let work_error_clone = m.clone();
        let sqs_client_or_else = self.sqs_client.clone(); // clone for if there was an error processing messages
        let sqs_client_and_then = self.sqs_client.clone(); // clone for handle_delete
        let worker = self.worker.clone();
        let worker_future = async { worker.process(message) };

        if let Err(e) = await!(worker_future) {
            await!(handle_work_error(
                sqs_client_or_else,
                e.clone(),
                work_error_clone
            ))
        } else {
            await!(handle_delete(sqs_client_and_then, delete_clone))
        }

        // await!(worker.process(message))
        //     .map_err(ProcessorError::from)
        //     .or_else(|ref pe| {
        //         async {
        //             if let ProcessorError::WorkErrorOccurred(we) = pe.clone() {
        //                 await!(handle_work_error(sqs_client_or_else, we.clone(), work_error_clone))
        //             } else {
        //                 Box::new(Err(ProcessorError::Unknown))
        //             }
        //         }
        //     })
        //     .and_then(|_f| async {
        //         await!(handle_delete(sqs_client_and_then, delete_clone)
        //     })
        //     .map_err(|e| error!("Error occurred: {}", e))
    }
}

// fn processor_error_future(pe: ProcessorError) -> Box<ProcessorErrorFuture> {
//     Box::new(err(pe))
// }

fn build_sqs_client(mode: &Mode) -> SqsClient {
    match mode {
        Mode::AWS(region, queue) => SqsClient::new(region.to_owned(), queue),
        Mode::Local(port, queue) => SqsClient::local(*port, queue),
    }
}

async fn handle_delete(sqs_client: SqsClient, message: SqsMessage) -> Result<(), ProcessorError> {
    if let Some(receipt_handle) = message.receipt_handle.clone() {
        await!(sqs_client.delete_message(receipt_handle.as_ref()))
    } else {
        error!("No receipt id found for message: {:?}", message);
        Ok(())
    }
}

async fn handle_requeue(sqs_client: SqsClient, message: SqsMessage) -> Result<(), ProcessorError> {
    await!(sqs_client.requeue(message, DEFAULT_REQUEUE_DELAY))
}

async fn handle_work_error(
    sqs_client: SqsClient,
    we: WorkError,
    m: SqsMessage,
) -> Result<(), ProcessorError> {
    let delete_clone = m.clone();
    match we.clone() {
        WorkError::UnRecoverableError(msg) => {
            error!("No way to recover from error: {} deleting", &msg);
            await!(handle_delete(sqs_client, delete_clone))
        }
        WorkError::RecoverableError(msg) => {
            error!("Recoverable from error: {} requeing", &msg);
            await!(handle_requeue(sqs_client, delete_clone))
        }
    }
}
