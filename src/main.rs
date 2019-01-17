#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

mod cli;
mod errors;
mod model;
mod sqs;

use crate::cli::{Cli, Mode};
use crate::sqs::SqsClient;
use env_logger;
use std::time::{Duration, Instant};
use tokio::executor::DefaultExecutor;
use tokio::executor::Executor;
use tokio::prelude::*;
use tokio::timer::Interval;

fn main() {
    env_logger::init();

    match Cli::new().determine_mode() {
        Ok(mode) => {
            let initialized_client = build_sqs_client(&mode);
            println!("Initializing rs-queue-processor: {:?}", &mode);

            // Clone required for the move in for_each. Cloning SqsClient is cheap as the underlying Rusoto client is embedded in an Arc
            let sqs_client = initialized_client.clone();
            let task = Interval::new(Instant::now(), Duration::from_secs(2))
                .for_each(move |_| {
                    debug!("Timer task is starting");
                    let sqs_client_2 = sqs_client.clone(); // This clone is required for the move in map
                    let f = sqs_client
                        .fetch_messages()
                        .map(move |messages| {
                            if messages.is_empty() {
                                println!("No messages received for queue")
                            } else {
                                for m in messages {
                                    if let Some(workload) = m.work_load.clone() {
                                        println!("Received message: {:#?}", workload.text)
                                    } else {
                                        println!(
                                            "Message {} had now workload ",
                                            m.message_id.clone()
                                                .unwrap_or_else(|| "<No ID Found>".to_string())
                                        )
                                    }
                                    if let Some(receipt_handle) = m.receipt_handle.clone() {
                                        let delete_future = sqs_client_2.delete_message(receipt_handle.as_ref())
                                            .map_err(|e| error!("An error occurred {}", e));
                                        let _r = DefaultExecutor::current().spawn(Box::new(delete_future));
                                    }
                                    else {
                                        warn!("Cannot delete message {:?} as it has no receipt handle", &m);
                                    }
                                }
                            }
                        })
                        .map_err(|e| panic!("An error occurred: {}", e));

                    let _r = DefaultExecutor::current().spawn(Box::new(f));
                    Ok(())
                })
                .map_err(|e| panic!("interval error; err={:#?}", e));

            tokio::run(task);
        }
        Err(e) => {
            panic!("{}", e);
        }
    }
}

fn build_sqs_client(mode: &Mode) -> SqsClient {
    match mode {
        Mode::AWS(region, queue) => SqsClient::new(region.to_owned(), queue),
        Mode::Local(port, queue) => SqsClient::local(*port, queue),
    }
}
