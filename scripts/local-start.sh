#!/bin/sh

cargo build 
RUST_LOG=main=debug cargo run --example=simple -- --local=9324 --queue="http://localhost:9324/queue/my-messages"
