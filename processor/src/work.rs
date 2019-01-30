use crate::errors::WorkError;
use futures::future::Future;
use rusoto_sqs::Message as SqsMessage;

pub type WorkerFuture = dyn Future<Item = (), Error = WorkError> + Send;

pub trait Worker {
    fn process(&self, message: SqsMessage) -> Box<WorkerFuture>;
}
