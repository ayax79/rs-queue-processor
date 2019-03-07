use crate::errors::ProcessorError;
use futures::future::Future as OldFuture;
use rusoto_core::HttpClient;
use rusoto_core::Region;
use rusoto_credential::StaticProvider;
use rusoto_sqs::{
    DeleteMessageRequest, Message as SqsMessage, ReceiveMessageRequest, SendMessageRequest, Sqs,
    SqsClient as RusotoSqsClient,
};
use std::convert::From;
use std::future::Future as NewFuture;
use std::sync::Arc;
use tokio_async_await::compat::forward::IntoAwaitable;

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

    pub fn fetch_messages(
        &self,
    ) -> impl NewFuture<Output = Result<Vec<SqsMessage>, ProcessorError>> {
        println!("fetch_messages called");
        let mut request = ReceiveMessageRequest::default();
        request.max_number_of_messages = Some(10);
        request.queue_url = self.queue_url.clone();

        self.sqs
            .receive_message(request)
            .map(|result| {
                println!("sqs: received message result: {:?}", &result);
                result.messages
            })
            .map(|maybe_messages| maybe_messages.unwrap_or_else(|| vec![]))
            .map_err(ProcessorError::from)
            .into_awaitable()
    }

    pub fn delete_message(
        &self,
        receipt_handle: &str,
    ) -> impl NewFuture<Output = Result<(), ProcessorError>> {
        debug!("delete_message called. receipt_handle: {}", receipt_handle);
        let mut request = DeleteMessageRequest::default();
        request.queue_url = self.queue_url.clone();
        request.receipt_handle = receipt_handle.to_owned();

        self.sqs
            .delete_message(request)
            .map(|_| ())
            .map_err(ProcessorError::from)
            .into_awaitable()
    }

    pub fn requeue(
        &self,
        message: SqsMessage,
        delay_seconds: i64,
    ) -> impl NewFuture<Output = Result<(), ProcessorError>> {
        let mut request = SendMessageRequest::default();
        request.queue_url = self.queue_url.to_owned();
        request.message_body = message.body.unwrap_or("".to_owned());
        request.delay_seconds = Some(delay_seconds);

        self.sqs
            .send_message(request)
            .map(|_| ())
            .map_err(ProcessorError::from)
            .into_awaitable()
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

fn build_local_region(port: u32) -> Region {
    Region::Custom {
        name: SQS_LOCAL_REGION.to_string(),
        endpoint: format!("http://localhost:{}", port),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusoto_sqs::{CreateQueueRequest, SendMessageRequest};
    use testcontainers::Docker;
    use testcontainers::{clients, images};
    //    use futures::executor::block_on;

    #[test]
    fn sqs_fetch_messages() {
        tokio::run_async(
            async {
                let docker = clients::Cli::default();
                let node = docker.run(images::elasticmq::ElasticMQ::default());
                let host_port = node.get_host_port(9324).unwrap();
                let region = build_local_region(host_port);
                let rusoto_sqs_client = build_sqs_client(region.clone());
                let queue_url = create_queue(&rusoto_sqs_client, create_queue_request());
                populate_queue(&rusoto_sqs_client, &queue_url);

                let client = SqsClient::local(host_port, queue_url.as_ref());

                let result: Vec<SqsMessage> = await!(client.fetch_messages()).unwrap();

                assert_eq!(1, result.len());
                let our_message: &SqsMessage = result.get(0).unwrap();
                assert_eq!("{\"text\": \"Hello\"}", our_message.body.clone().unwrap());

                let receipt_handle = our_message.receipt_handle.clone().unwrap();
                let result = await!(client.delete_message(receipt_handle.as_ref()));
                assert!(result.is_ok());
            },
        );
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
