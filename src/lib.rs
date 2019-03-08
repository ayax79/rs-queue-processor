#![feature(await_macro, async_await, futures_api)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate tokio;

pub mod config;
pub mod errors;
pub mod processor;
mod sqs;
pub mod work;
