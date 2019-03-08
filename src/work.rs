use crate::errors::WorkError;
use rusoto_sqs::Message as SqsMessage;

/// Trait to implement to create your own Worker implementation
pub trait Worker {

    /// Processes the specified sqs message
    /// 
    /// A WorkError can be returned on failture of message
    /// If the error is unrecoverable a WorkError::UnrecoverableError should be returned
    /// If the message should be requeued for later a WorkError::RecoverableError can be returned
    fn process(&self, message: SqsMessage) -> Result<(), WorkError>;
}
