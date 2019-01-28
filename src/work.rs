use crate::errors::WorkError;
use crate::model::Message;
use futures::future::{lazy, ok, Future};

type WorkerFuture = Future<Item = (), Error = WorkError> + Send;

pub trait Worker {
    fn process(&self, message: Message) -> Box<WorkerFuture>;
}

#[derive(Clone, Default)]
pub struct WorkerImpl;

impl Worker for WorkerImpl {
    fn process(&self, m: Message) -> Box<WorkerFuture> {
        let message = m.clone();
        let maybe_workload = message.work_load.clone();
        Box::new(lazy(move || {
            if let Some(workload) = maybe_workload {
                println!("Received message: {:#?}", workload.text)
            } else {
                println!(
                    "Message {} had now workload ",
                    message
                        .message_id
                        .clone()
                        .unwrap_or_else(|| "<No ID Found>".to_string())
                )
            }
            ok(())
        }))
    }
}
