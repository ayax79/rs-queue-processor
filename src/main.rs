#[macro_use]
extern crate serde_derive;

//#[macro_use]
//extern crate log;

mod cli;
mod errors;
mod model;
mod sqs;

use crate::errors::ProcessorError::{self, CommandLineError};
use crate::sqs::SqsClient;
use crate::cli::{Cli, Mode};
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::{Interval, Error as TimerError};
use tokio::executor::DefaultExecutor;
use tokio::executor::Executor;

use rusoto_core::Region;

fn main() {
    match Cli::new().determine_mode() {
        Ok(mode) => {
            let initialized_client = build_sqs_client(&mode);

            let sqs_client = initialized_client.clone();
            let task = Interval::new(Instant::now(), Duration::from_secs(2))
                .take(10)
                .for_each(move |_| {
                    println!("Timer task is starting");
                    let f = sqs_client.fetch_messages()
                        .map(|m| {
                            println!("Received message: {:#?}", m)
                        })
                        .map_err(|e| {
                            panic!("An error occurred: {:#?}", e)

                        });

                    DefaultExecutor::current().spawn(Box::new(f));
                    Ok(())
                })
                .map_err(|e| panic!("interval error; err={:#?}", e));

            DefaultExecutor::current().spawn(Box::new(task));
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn build_sqs_client(mode: &Mode) -> SqsClient {
    match mode {
        Mode::AWS(region, queue) => SqsClient::new(region.to_owned(), queue),
        Mode::Local(port, queue) => SqsClient::local(*port, queue),
    }
}
