use rs_queue_processor::config::{Config, Mode};
use rs_queue_processor::processor::Processor;
use rusoto_core::Region;
use rusoto_sqs::{
    CreateQueueRequest, SendMessageRequest, Sqs, SqsClient as RusotoSqsClient,
};
use std::sync::mpsc::{self, SyncSender, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use testcontainers::images::elasticmq::ElasticMQ;
use testcontainers::{clients, Docker};

use rs_queue_processor::errors::WorkError;
use rs_queue_processor::work::Worker;
use rusoto_sqs::Message;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Action {
    Success,
    FailRequeue,
    FailDelete,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Payload {
    msg: String,
    action: Action,
}

impl Payload {
    pub fn new(msg: &str, action: Action) -> Self {
        Payload {
            msg: msg.to_owned(),
            action,
        }
    }
}

pub struct TestWorker {
    sender: SyncSender<Payload>,
}

impl TestWorker {
    pub fn new(sender: SyncSender<Payload>) -> Self {
        println!("TestWorker created");
        TestWorker { sender }
    }
}

impl Worker for TestWorker {
    fn process(&self, message: Message) -> Result<(), WorkError> {
        println!("Worker process called!");
        let sender = mpsc::SyncSender::clone(&self.sender);
        message
            .body
            .ok_or(WorkError::UnRecoverableError("No Body Found".to_owned()))
            .and_then(|body| {
                println!("Work found message with body: {:?}", &body);
                serde_json::from_str::<Payload>(body.as_ref()).map_err(|e| {
                    WorkError::UnRecoverableError(format!("Body does not match payload: {:?}", e))
                })
            })
            .and_then(move |payload| {
                println!("Receiving {:?}", &payload);
                let send_result = sender.send(payload.clone());
                println!("Sending Result: {:?}", send_result);
                match payload.action {
                    Action::Success => Ok(()),
                    Action::FailRequeue => Err(WorkError::RecoverableError(
                        "Received requeue action".to_owned(),
                    )),
                    Action::FailDelete => Err(WorkError::UnRecoverableError(
                        "Received delete action".to_owned(),
                    )),
                }
            })
    }
}

pub fn with_processor_util<F>(f: F) 
    where F: FnOnce(ProcessorUtil) -> () 
{
        let (tx, rx) = mpsc::sync_channel::<Payload>(1);
        println!("Creating Channel");

        let queue_name = "test-queue";
        let docker = clients::Cli::default();
        let node = docker.run(ElasticMQ::default());
        let host_port = node.get_host_port(9324).unwrap();
        let region = build_local_region(host_port);
        let sqs_client = build_sqs_client(host_port);

        println!("Creating queue");

        let queue_url = create_queue(Arc::clone(&sqs_client), queue_name.to_owned()).unwrap();
        let config = Config::default().with_mode(Mode::AWS(region, queue_url.to_owned()));

        println!("Queue successfully created: {:?}", &queue_url);
        let worker = TestWorker::new(tx);

        let mut processor = Processor::new(&config, Box::new(worker)).unwrap();
        processor.start();

        f(ProcessorUtil::new(rx, Arc::clone(&sqs_client), queue_url.clone()));

        processor.stop().unwrap();
}


pub struct ProcessorUtil {
    rx: Receiver<Payload>,
    queue_url: String,
    sqs_client: Arc<RusotoSqsClient>,
}

impl ProcessorUtil {

    fn new(rx: Receiver<Payload>, sqs_client: Arc<RusotoSqsClient>, queue_url: String) -> Self {
        ProcessorUtil {
            rx,
            queue_url,
            sqs_client,
        }
    }

    pub fn send_payload(&self, payload: Payload) {
        send_message(Arc::clone(&self.sqs_client), self.queue_url.clone(), payload.clone()).unwrap();
    }

    pub fn wait_for_payload(&self, duration: Duration) -> Result<Payload, RecvTimeoutError> {
        self.rx.recv_deadline(Instant::now() + duration)
    }

}

fn send_message(
    client: Arc<RusotoSqsClient>,
    queue_url: String,
    payload: Payload,
) -> Result<(), String> {
    let json = serde_json::to_string(&payload).map_err(|e| {
        eprintln!("Payload cannot be serialized: {:?}", e);
        format!("send_message error: {:?}", e)
    })?;

    let mut request = SendMessageRequest::default();
    request.queue_url = queue_url.to_owned();
    request.message_body = json;
    client
        .send_message(request)
        .sync()
        .map(|result| {
            println!("send message result: {:?}", &result);
            ()
        })
        .map_err(|e| {
            eprintln!("Could not send message: {:?}", e);
            format!("send_message error: {:?}", e)
        })
}

fn create_queue(client: Arc<RusotoSqsClient>, queue_name: String) -> Result<String, ()> {
    println!("create_queue called!");
    let mut request = CreateQueueRequest::default();
    request.queue_name = queue_name;

    client
        .create_queue(request)
        .sync()
        .map(|result| {
            println!("create_queue result: {:?}", &result);
            result.queue_url.unwrap()
        })
        .map_err(|e| panic!("Could not create queue {:?}", e))
}

fn build_local_region(port: u32) -> Region {
    Region::Custom {
        name: "local".to_owned(),
        endpoint: format!("http://localhost:{}", port),
    }
}

fn build_sqs_client(port: u32) -> Arc<RusotoSqsClient> {
    Arc::new(RusotoSqsClient::new(build_local_region(port)))
}



