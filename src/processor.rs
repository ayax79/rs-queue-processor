use crate::config::{Config, Mode};
use crate::errors::{ProcessorError, WorkError};
use crate::sqs::SqsClient;
use crate::work::Worker;
use rusoto_sqs::Message as SqsMessage;
use std::sync::Arc;
use std::time::{Duration, Instant};
use log::{debug, error, info, trace};
use tokio::timer::Interval;

// todo: make configurable
/// Default requeue delay in seconds
const DEFAULT_REQUEUE_DELAY: i64 = 10;

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
        info!("Initializing rs-queue-processor: {:?}", &config.mode);
        let sqs_client = build_sqs_client(&config.mode);

        // let runtime = Builder::new()
        //     .blocking_threads(4)
        //     .core_threads(4)
        //     .keep_alive(Some(Duration::from_secs(60)))
        //     .name_prefix("rs-queue-processor-")
        //     .stack_size(3 * 1024 * 1024)
        //     .build()
        //     .map_err(ProcessorError::from)?;

        Ok(Processor {
            sqs_client,
            worker: Arc::from(worker),
        })
    }

    /// Generates a Interval Task that can be executed
    ///
    /// let processor = Processor::new(&config, worker);
    /// tokio::run(processor.process());
    pub async fn process(&self)  {
        trace!("process called!!");
        // Clone required for the move in for_each. Cloning SqsClient is cheap as the underlying Rusoto client is embedded in an Arc
        let self_clone = self.clone();
        let mut interval = Interval::new(Instant::now(), Duration::from_millis(100));
        while let Some(instant) = interval.next().await {
            trace!("Timer task is starting: instant {:?}", &instant);
            let clone_2 = self_clone.clone();
            let _r = tokio::spawn(
                async move {
                    clone_2.process_messages().await;
                }
            );
        }
            // .map_err(|e| panic!("interval error; err={:#?}", e));
    }

    /// Returns a future that will fetch messages from
    /// SQS to be processed
    async fn process_messages(&self) {
        trace!("process_messages called!");
        match self.sqs_client.fetch_messages().await {
            Ok(messages) => {
                debug!("fetch messages result: {:?}", &messages);
                for message in messages {
                    debug!("process_messages: handling {:?}", &message);
                    let result = self.process_message(message.clone()).await;
                    if let Err(e) = result {
                        error!("Error processing message: {:?} error: {}", &message, &e);
                    }
                }
            }
            Err(err) => error!("Error fetching messages: {}", err),
        }
    }

    /// Returns a future that will process one message
    /// The message will be passed to the worker.
    async fn process_message(&self, m: SqsMessage) -> Result<(), ProcessorError> {
        debug!("Process message called with: {:?}", &m);
        let message = m.clone();
        let delete_clone = m.clone();
        let work_error_clone = m.clone();
        let sqs_client_or_else = self.sqs_client.clone(); // clone for if there was an error processing messages
        let sqs_client_and_then = self.sqs_client.clone(); // clone for handle_delete
        let worker = self.worker.clone();
        let worker_future = async { worker.process(message) };

        if let Err(e) = worker_future.await {
            trace!("Received work error: {:?}", &e);
            handle_work_error(
                sqs_client_or_else,
                e.clone(),
                work_error_clone
            ).await
        } else {
            handle_delete(sqs_client_and_then, delete_clone).await
        }
    }
}

fn build_sqs_client(mode: &Mode) -> SqsClient {
    match mode {
        Mode::AWS(region, queue) => SqsClient::new(region.to_owned(), queue),
        Mode::Local(port, queue) => SqsClient::local(*port, queue),
    }
}

async fn handle_delete(sqs_client: SqsClient, message: SqsMessage) -> Result<(), ProcessorError> {
    if let Some(receipt_handle) = message.receipt_handle.clone() {
        sqs_client.delete_message(receipt_handle.as_ref()).await
    } else {
        error!("No receipt id found for message: {:?}", message);
        Ok(())
    }
}

async fn handle_requeue(sqs_client: SqsClient, message: SqsMessage) -> Result<(), ProcessorError> {
    sqs_client.requeue(message, DEFAULT_REQUEUE_DELAY).await
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
            handle_delete(sqs_client, delete_clone).await
        }
        WorkError::RecoverableError(msg) => {
            error!("Recoverable from error: {} requeing", &msg);
            handle_requeue(sqs_client, delete_clone).await
        }
    }
}
