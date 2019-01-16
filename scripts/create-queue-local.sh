#!/bin/sh
export HOSTNAME_EXTERNAL="http://localhost:9324"
aws --endpoint-url $HOSTNAME_EXTERNAL sqs create-queue --queue-name my-messages 