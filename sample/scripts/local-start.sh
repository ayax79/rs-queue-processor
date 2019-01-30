#!/bin/sh

cargo build 
RUST_LOG=main=debug ../target/debug/rs-queue-processor-sample-app --local=9324 --queue="http://localhost:9324/queue/my-messages"