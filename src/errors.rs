use rusoto_core::request::HttpDispatchError as RusotoHttpDispatchError;
use rusoto_credential::CredentialsError as RusotoCredentialsError;
use rusoto_sqs::{DeleteMessageError, ReceiveMessageError, SendMessageError};
use std::convert::From;
use std::error::Error;
use std::fmt::{self, Display};
use std::io::Error as IOError;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ProcessorError {
    IOError(Arc<IOError>),
    SqsReceiveMessageError(Arc<ReceiveMessageError>),
    SqsDeleteMessageError(Arc<DeleteMessageError>),
    SqsSendMessageError(Arc<SendMessageError>),
    CredentialsError(Arc<RusotoCredentialsError>),
    HttpDispatchError(Arc<RusotoHttpDispatchError>),
    CommandLineError(&'static str),
    WorkErrorOccurred(WorkError),
    Unknown,
}

impl<'a> Display for ProcessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessorError::IOError(e) => write!(f, "An std::io::Error occurred: {}", e),
            ProcessorError::SqsReceiveMessageError(e) => match e.deref() {
                ReceiveMessageError::Unknown(be) => {
                    let message = String::from_utf8_lossy(be.body.as_slice());
                    write!(f, "Unknown Error receiving SQS message: {:#?}", message)
                }
                _ => write!(f, "Error receiving SQS message: {}", e),
            },
            ProcessorError::SqsDeleteMessageError(e) => write!(
                f,
                "An error occurred when attempted to delete a message {}",
                e
            ),
            ProcessorError::CredentialsError(e) => write!(f, "A credentials error occurred: {}", e),
            ProcessorError::HttpDispatchError(e) => {
                write!(f, "An HttpDispatch Error occurred: {}", e)
            }
            ProcessorError::CommandLineError(e) => {
                write!(f, "A command line error occurred: {}", e)
            }
            ProcessorError::Unknown => write!(f, "An unknown error occurred"),
            ProcessorError::WorkErrorOccurred(e) => write!(f, "A work error occurred: {}", e),
            ProcessorError::SqsSendMessageError(e) => write!(f, "Error Sending message {}", e),
        }
    }
}

impl Error for ProcessorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            ProcessorError::IOError(ref e) => Some(e.as_ref()),
            ProcessorError::SqsReceiveMessageError(ref e) => Some(e.as_ref()),
            ProcessorError::CredentialsError(ref e) => Some(e.as_ref()),
            ProcessorError::HttpDispatchError(ref e) => Some(e.as_ref()),
            ProcessorError::SqsDeleteMessageError(ref e) => Some(e.as_ref()),
            ProcessorError::WorkErrorOccurred(ref we) => Some(we),
            ProcessorError::SqsSendMessageError(ref e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<IOError> for ProcessorError {
    fn from(e: IOError) -> Self {
        ProcessorError::IOError(Arc::new(e))
    }
}

impl From<ReceiveMessageError> for ProcessorError {
    fn from(e: ReceiveMessageError) -> Self {
        ProcessorError::SqsReceiveMessageError(Arc::new(e))
    }
}

impl From<RusotoCredentialsError> for ProcessorError {
    fn from(e: RusotoCredentialsError) -> Self {
        ProcessorError::CredentialsError(Arc::new(e))
    }
}

impl From<RusotoHttpDispatchError> for ProcessorError {
    fn from(e: RusotoHttpDispatchError) -> Self {
        ProcessorError::HttpDispatchError(Arc::new(e))
    }
}

impl From<DeleteMessageError> for ProcessorError {
    fn from(e: DeleteMessageError) -> Self {
        ProcessorError::SqsDeleteMessageError(Arc::new(e))
    }
}

impl From<SendMessageError> for ProcessorError {
    fn from(e: SendMessageError) -> Self {
        ProcessorError::SqsSendMessageError(Arc::new(e))
    }
}

impl From<WorkError> for ProcessorError {
    fn from(e: WorkError) -> Self {
        ProcessorError::WorkErrorOccurred(e)
    }
}

type WorkErrorMessage = String;

#[derive(Debug, Clone)]
pub enum WorkError {
    #[allow(dead_code)]
    RecoverableError(WorkErrorMessage),
    #[allow(dead_code)]
    UnRecoverableError(WorkErrorMessage),
}

impl<'a> Display for WorkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkError::RecoverableError(msg) => write!(f, "A recoverable error occurred: {}", msg),
            WorkError::UnRecoverableError(msg) => {
                write!(f, "A unrecoverable error occurred: {}", msg)
            }
        }
    }
}

impl Error for WorkError {}
