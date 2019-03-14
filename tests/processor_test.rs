#![feature(await_macro, async_await, futures_api, deadline_api)]

extern crate rs_queue_processor;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod processor_util;

use crate::processor_util::{with_processor_util, Action, Payload};
use std::time::Duration;

macro_rules! assert_received {
    ($pu:expr, $duration:expr, $payload:expr) => {{
        match $pu.wait_for_payload($duration) {
            Ok(result) => assert_eq!($payload, result),
            Err(_) => panic!("Timedout out waiting for response"),
        }
    }};
}

#[test]
fn test_success() {
    with_processor_util(|pu| {
        let payload = Payload::new("This message should be successful", Action::Success);
        pu.send_payload(payload.clone());
        assert_received!(pu, Duration::from_secs(5), payload);
    });
}

#[test]
fn test_fail_delete() {
    with_processor_util(|pu| {
        let payload = Payload::new(
            "This message should fail and then be deleted",
            Action::FailDelete,
        );
        pu.send_payload(payload.clone());
        assert_received!(pu, Duration::from_secs(5), payload);
    });
}

#[test]
fn test_fail_requeue() {
    with_processor_util(|pu| {
        let payload = Payload::new(
            "This message should fail and then be requeued",
            Action::FailRequeue,
        );
        pu.send_payload(payload.clone());
        assert_received!(pu, Duration::from_secs(5), payload);
        assert_received!(pu, Duration::from_secs(20), payload);
    });
}
