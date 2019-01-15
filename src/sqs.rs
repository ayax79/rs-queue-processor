use crate::errors::ProcessorError;
use crate::model::OurMessage;
use futures::future::Future;
use rusoto_core::Region;
use rusoto_sqs::{
    Message, ReceiveMessageRequest, Sqs, SqsClient as RusotoSqsClient,
};
use std::convert::From;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
struct SqsClient {
    queue_url: String,
    sqs: Arc<RusotoSqsClient>,
}

impl SqsClient {
    pub fn new(region: Region, queue_url: &str) -> Self {
        let sqs = RusotoSqsClient::new(region);
        SqsClient {
            queue_url: queue_url.to_owned(),
            sqs: Arc::new(sqs),
        }
    }

    #[cfg(test)]
    pub fn new_with_rusoto_client(rusoto_client: RusotoSqsClient, queue_url: &str) -> Self {
        SqsClient {
            queue_url: queue_url.to_owned(),
            sqs: Arc::new(rusoto_client)
        }
    }

    pub fn fetch_messages(&self) -> impl Future<Item = Vec<OurMessage>, Error = ProcessorError> {
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
}

fn messages_to_our_messages(messages: &Vec<Message>) -> Vec<OurMessage> {
    messages
        .iter()
        .flat_map(|m| {
            message_to_our_message(m)
                .map(|m| vec![m])
                .unwrap_or_else(|| vec![])
        })
        .collect()
}

fn message_to_our_message(message: &Message) -> Option<OurMessage> {
    message
        .body
        .clone()
        .and_then(|b| OurMessage::from_str(b.as_ref()).ok())
}


#[cfg(test)]
mod tests {
    use super::*;
    use rusoto_core::HttpClient;
    use testcontainers::{clients, images};
    use testcontainers::clients::Cli;
    use testcontainers::Docker;
    use rusoto_sqs::{CreateQueueRequest, SendMessageRequest};
    use rusoto_credential::StaticProvider;
    use rusoto_core::RusotoFuture;

    #[test]
    fn sqs_fetch_messages() {
        let docker = clients::Cli::default();
        let node = docker.run(images::elasticmq::ElasticMQ::default());
        let host_port = node.get_host_port(9324).unwrap();
        let region = build_region(host_port);
        let rusoto_sqs_client = build_sqs_client(region.clone());
        let queue_url = create_queue(&rusoto_sqs_client, create_queue_request());
        populate_queue(&rusoto_sqs_client, &queue_url);


        let client = SqsClient::new_with_rusoto_client(rusoto_sqs_client, queue_url.as_ref());

        client.fetch_messages();

        let result: Vec<OurMessage> =
            RusotoFuture::from_future(client.fetch_messages()).sync().unwrap();

        assert_eq!(1, result.len());
        let our_message: &OurMessage = result.get(0).unwrap();
        assert_eq!("Hello", our_message.text);
    }

    fn populate_queue(client: &RusotoSqsClient, queue_url: &str) {
        let mut request = SendMessageRequest::default();
        request.queue_url = queue_url.to_owned();
        request.message_body = r#"{"text": "Hello"}"#.to_owned();

        client.send_message(request).sync().unwrap();
    }

    fn build_sqs_client(region: Region) -> RusotoSqsClient {
        let dispatcher = HttpClient::new().expect("could not create http client");
        let credentials_provider =
            StaticProvider::new("fakeKey".to_string(), "fakeSecret".to_string(), None, None);
        RusotoSqsClient::new_with(dispatcher, credentials_provider, region)
    }

    fn build_region(host_port: u32) -> Region {
        Region::Custom {
            name: "sqs-local".to_string(),
            endpoint: format!("http://localhost:{}", host_port),
        }
    }

    fn create_queue(client: &RusotoSqsClient, request: CreateQueueRequest) -> String {
        let response = client.create_queue(request)
            .sync()
            .unwrap();

        response.queue_url.unwrap()
    }

    fn create_queue_request() -> CreateQueueRequest {
        let mut request = CreateQueueRequest::default();
        request.queue_name = "our-messages".to_owned();
        request
    }
}