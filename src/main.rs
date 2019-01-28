#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

mod config;
mod errors;
mod model;
mod processor;
mod sqs;
mod work;

use crate::config::Cli;
use crate::processor::Processor;
use crate::work::WorkerImpl;
use env_logger;

fn main() {
    env_logger::init();

    match Cli::new().determine_mode() {
        Ok(mode) => {
            let worker = WorkerImpl::default();
            let processor = Processor::new(&mode, Box::new(worker));
            processor.unwrap().process();
        }
        Err(e) => {
            panic!("{}", e);
        }
    }
}
