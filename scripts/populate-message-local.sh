#!/bin/sh

if [ "$#" -ne 1 ]; then
    ( >&2 echo "No message specified!" )
fi

export HOSTNAME_EXTERNAL="http://localhost:9324"
QUEUE_URL="http://localhost:9324/queue/my-messages"
aws --endpoint-url $HOSTNAME_EXTERNAL sqs send-message \
    --queue-url $QUEUE_URL \
    --message-body "{\"text\": \"$1\"}"