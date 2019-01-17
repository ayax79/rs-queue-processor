use rusoto_core::request::HttpDispatchError as RusotoHttpDispatchError;
use rusoto_credential::CredentialsError as RusotoCredentialsError;
use rusoto_sqs::{DeleteMessageError, ReceiveMessageError};
use serde_json::Error as SerdeJsonError;
use std::convert::From;
use std::error::Error;
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum ProcessorError {
    JsonParseError(SerdeJsonError),
    SqsReceiveMessageError(ReceiveMessageError),
    SqsDeleteMessageError(DeleteMessageError),
    CredentialsError(RusotoCredentialsError),
    HttpDispatchError(RusotoHttpDispatchError),
    CommandLineError(&'static str),
}

impl<'a> Display for ProcessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessorError::JsonParseError(e) => write!(f, "Error parsing JSON: {:#?}", e),
            ProcessorError::SqsReceiveMessageError(e) => match e {
                ReceiveMessageError::Unknown(be) => {
                    let message = String::from_utf8_lossy(be.body.as_slice());
                    write!(f, "Unknown Error receiving SQS message: {:#?}", message)
                }
                _ => write!(f, "Error receiving SQS message: {:#?}", e),
            },
            ProcessorError::SqsDeleteMessageError(e) => write!(
                f,
                "An error occurred when attempted to delete a message {:#?}",
                e
            ),
            ProcessorError::CredentialsError(e) => {
                write!(f, "A credentials error occurred: {:#?}", e)
            }
            ProcessorError::HttpDispatchError(e) => {
                write!(f, "An HttpDispatch Error occurred: {:#?}", e)
            }
            ProcessorError::CommandLineError(e) => {
                write!(f, "A command line error occurred: {}", e)
            }
        }
    }
}

impl Error for ProcessorError {
    fn source(&self) -> Option<&(Error + 'static)> {
        match *self {
            ProcessorError::JsonParseError(ref e) => Some(e),
            ProcessorError::SqsReceiveMessageError(ref e) => Some(e),
            ProcessorError::CredentialsError(ref e) => Some(e),
            ProcessorError::HttpDispatchError(ref e) => Some(e),
            ProcessorError::SqsDeleteMessageError(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<SerdeJsonError> for ProcessorError {
    fn from(e: SerdeJsonError) -> Self {
        ProcessorError::JsonParseError(e)
    }
}

impl From<ReceiveMessageError> for ProcessorError {
    fn from(e: ReceiveMessageError) -> Self {
        ProcessorError::SqsReceiveMessageError(e)
    }
}

impl From<RusotoCredentialsError> for ProcessorError {
    fn from(e: RusotoCredentialsError) -> Self {
        ProcessorError::CredentialsError(e)
    }
}

impl From<RusotoHttpDispatchError> for ProcessorError {
    fn from(e: RusotoHttpDispatchError) -> Self {
        ProcessorError::HttpDispatchError(e)
    }
}

impl From<DeleteMessageError> for ProcessorError {
    fn from(e: DeleteMessageError) -> Self {
        ProcessorError::SqsDeleteMessageError(e)
    }
}

#[derive(Debug, Clone)]
pub struct NoSQSBodyException;

impl Error for NoSQSBodyException {}

impl<'a> Display for NoSQSBodyException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The sqs message did not contain a body")
    }
}
