use crate::errors::WorkError;
use futures::future::Future;
use rusoto_sqs::Message as SqsMessage;

pub type WorkerFuture = dyn Future<Item = (), Error = WorkError> + Send;

/// Trait to implement to create your own Worker implementation
pub trait Worker {
    /// Creates the future for processing a work message
    ///
    /// A WorkError can be returned on failture of message
    /// If the error is unrecoverable a WorkError::UnrecoverableError should be returned
    /// If the message should be requeued for later a WorkError::RecoverableError can be returned
    fn process(&self, message: SqsMessage) -> Box<WorkerFuture>;
}
