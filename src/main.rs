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
use std::time::{Duration, Instant};
use tokio::executor::DefaultExecutor;
use tokio::executor::Executor;
use tokio::prelude::*;
use tokio::timer::Interval;
use env_logger;

fn main() {
    env_logger::init();

    match Cli::new().determine_mode() {
        Ok(mode) => {
            let initialized_client = build_sqs_client(&mode);
            info!("Initializing rs-queue-processor: {:?}", &mode);

            let sqs_client = initialized_client.clone();
                let task = Interval::new(Instant::now(), Duration::from_secs(2))
                    .for_each(move |_| {
                    debug!("Timer task is starting");
                    let f = sqs_client
                        .fetch_messages()
                        .map(|messages| {
                            if messages.is_empty() {
                                debug!("No messages received for queue")
                            } else {
                                for m in messages {
                                    debug!("Received message: {:#?}", m.text)
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
