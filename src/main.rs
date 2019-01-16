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
//use tokio::io;
//use tokio::net::TcpStream;
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Interval;

use rusoto_core::Region;

fn main() {}
