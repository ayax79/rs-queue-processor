#![feature(await_macro, async_await, futures_api, deadline_api)]

extern crate rs_queue_processor;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod processor_util;

use std::time::Duration;
use crate::processor_util::{Payload, Action, with_processor_util};

macro_rules! assert_received {
    ($pu:expr, $duration:expr, $payload:expr) => ({
        match $pu.wait_for_payload($duration) {
            Ok(result) => assert_eq!($payload, result),
            Err(_) => {
                panic!("Timedout out waiting for response")
            }
        }
    })
}

#[test]
fn test_success() {
    with_processor_util(|pu| {
        let payload = Payload::new("This message should be successful", Action::Success);
        pu.send_payload(payload.clone());
        println!("Waiting for result");
        assert_received!(pu, Duration::from_millis(5000), payload);
    });
}

#[test]
fn test_fail_delete() {
    with_processor_util(|pu| {
        let payload = Payload::new("This message should fail and then be deleted", Action::FailDelete);
        pu.send_payload(payload.clone());
        println!("Waiting for result");
        assert_received!(pu, Duration::from_millis(5000), payload);
    });
}

