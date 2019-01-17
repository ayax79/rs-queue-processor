#!/bin/sh

RUST_LOG=main=debug cargo run -- --local=9324 --queue="http://localhost:9324/queue/my-messages"