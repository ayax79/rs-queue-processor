use crate::errors::ProcessorError;
use crate::model::{Message, WorkLoad};
use futures::future::Future;
use rusoto_core::HttpClient;
use rusoto_core::Region;
use rusoto_credential::StaticProvider;
use rusoto_sqs::{
    DeleteMessageRequest, Message as SqsMessage, ReceiveMessageRequest, Sqs,
    SqsClient as RusotoSqsClient,
};
use std::convert::From;
use std::str::FromStr;
use std::sync::Arc;

const SQS_LOCAL_REGION: &'static str = "sqs-local";

#[derive(Clone)]
pub struct SqsClient {
    pub queue_url: String,
    sqs: Arc<RusotoSqsClient>,
}

impl SqsClient {
    pub fn new(region: Region, queue_url: &str) -> Self {
        let sqs = build_sqs_client(region);
        SqsClient {
            queue_url: queue_url.to_owned(),
            sqs: Arc::new(sqs),
        }
    }

    pub fn local(port: u32, queue_url: &str) -> Self {
        SqsClient::new(build_local_region(port), queue_url)
    }

    pub fn fetch_messages(&self) -> impl Future<Item = Vec<Message>, Error = ProcessorError> {
        debug!("fetch_messages called");
        let mut request = ReceiveMessageRequest::default();
        request.max_number_of_messages = Some(10);
        request.queue_url = self.queue_url.clone();

        self.sqs
            .receive_message(request)
            .map(|result| result.messages)
            .map(|maybe_messages| maybe_messages.unwrap_or_else(|| vec![]))
            .map(|messages| messages_to_our_messages(&messages))
            .map_err(ProcessorError::from)
    }

    pub fn delete_message(
        &self,
        receipt_handle: &str,
    ) -> impl Future<Item = (), Error = ProcessorError> {
        debug!("delete_message called. receipt_handle: {}", receipt_handle);
        let mut request = DeleteMessageRequest::default();
        request.queue_url = self.queue_url.clone();
        request.receipt_handle = receipt_handle.to_owned();

        self.sqs
            .delete_message(request)
            .map(|_| ())
            .map_err(ProcessorError::from)
    }
}

fn build_sqs_client(region: Region) -> RusotoSqsClient {
    match region {
        Region::Custom {
            name: _,
            endpoint: _,
        } => {
            let dispatcher = HttpClient::new().expect("could not create http client");
            let credentials_provider =
                StaticProvider::new("fakeKey".to_string(), "fakeSecret".to_string(), None, None);
            RusotoSqsClient::new_with(dispatcher, credentials_provider, region)
        }
        _ => RusotoSqsClient::new(region),
    }
}

fn messages_to_our_messages(messages: &Vec<SqsMessage>) -> Vec<Message> {
    messages.iter().map(message_to_our_message).collect()
}

fn message_to_our_message(message: &SqsMessage) -> Message {
    message
        .body
        .clone()
        .map(|b| {
            let result = WorkLoad::from_str(b.as_ref());
            if let &Err(ref e) = &result {
                error!("Error parsing message of {}", &e);
            }

            Message::new(
                message.receipt_handle.clone(),
                message.message_id.clone(),
                result.ok(),
            )
        })
        .unwrap_or_else(|| {
            Message::new(
                message.receipt_handle.clone(),
                message.message_id.clone(),
                None,
            )
        })
}

fn build_local_region(port: u32) -> Region {
    Region::Custom {
        name: SQS_LOCAL_REGION.to_string(),
        endpoint: format!("http://localhost:{}", port),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusoto_core::RusotoFuture;
    use rusoto_sqs::{CreateQueueRequest, SendMessageRequest};
    use testcontainers::Docker;
    use testcontainers::{clients, images};

    #[test]
    fn sqs_fetch_messages() {
        let docker = clients::Cli::default();
        let node = docker.run(images::elasticmq::ElasticMQ::default());
        let host_port = node.get_host_port(9324).unwrap();
        let region = build_local_region(host_port);
        let rusoto_sqs_client = build_sqs_client(region.clone());
        let queue_url = create_queue(&rusoto_sqs_client, create_queue_request());
        populate_queue(&rusoto_sqs_client, &queue_url);

        let client = SqsClient::local(host_port, queue_url.as_ref());

        client.fetch_messages();

        let result: Vec<Message> = RusotoFuture::from_future(client.fetch_messages())
            .sync()
            .unwrap();

        assert_eq!(1, result.len());
        let our_message: &Message = result.get(0).unwrap();
        let workload = our_message.work_load.clone().unwrap();
        assert_eq!("Hello", workload.text);

        let receipt_handle = our_message.receipt_handle.clone().unwrap();
        let result =
            RusotoFuture::from_future(client.delete_message(receipt_handle.as_ref())).sync();
        assert!(result.is_ok());
    }

    fn populate_queue(client: &RusotoSqsClient, queue_url: &str) {
        let mut request = SendMessageRequest::default();
        request.queue_url = queue_url.to_owned();
        request.message_body = r#"{"text": "Hello"}"#.to_owned();

        client.send_message(request).sync().unwrap();
    }

    fn create_queue(client: &RusotoSqsClient, request: CreateQueueRequest) -> String {
        let response = client.create_queue(request).sync().unwrap();

        response.queue_url.unwrap()
    }

    fn create_queue_request() -> CreateQueueRequest {
        let mut request = CreateQueueRequest::default();
        request.queue_name = "our-messages".to_owned();
        request
    }
}
