extern crate rs_queue_processor;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use futures::future::{lazy, result, Future};
use rs_queue_processor::config::{Config, Mode};
use rs_queue_processor::errors::WorkError;
use rs_queue_processor::processor::Processor;
use rs_queue_processor::work::Worker;
use rusoto_core::{Region, RusotoFuture};
use rusoto_sqs::{
    CreateQueueRequest, Message, SendMessageRequest, Sqs, SqsClient as RusotoSqsClient,
};
use std::sync::mpsc::{self, SyncSender};
use std::sync::Arc;
use std::time::Duration;
use testcontainers::images::elasticmq::ElasticMQ;
use testcontainers::{clients, Docker};
use tokio::executor::DefaultExecutor;
use tokio::executor::Executor;
use tokio::runtime::Runtime;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
enum Action {
    Success,
    FailRequeue,
    FailDelete,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
struct Payload {
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

struct TestWorker {
    sender: SyncSender<Payload>,
}

impl TestWorker {
    fn new(sender: SyncSender<Payload>) -> Self {
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

fn build_local_region(port: u32) -> Region {
    Region::Custom {
        name: "local".to_owned(),
        endpoint: format!("http://localhost:{}", port),
    }
}

fn build_sqs_client(port: u32) -> Arc<RusotoSqsClient> {
    Arc::new(RusotoSqsClient::new(build_local_region(port)))
}

fn send_message(
    client: Arc<RusotoSqsClient>,
    queue_url: String,
    payload: Payload,
) -> impl Future<Item = (), Error = String> {
    result(serde_json::to_string(&payload).map_err(|e| {
        eprintln!("Payload cannot be serialized: {:?}", e);
        format!("send_message error: {:?}", e)
    }))
    .and_then(move |json| {
        let mut request = SendMessageRequest::default();
        request.queue_url = queue_url.to_owned();
        request.message_body = json;
        client
            .send_message(request)
            .map(|result| {
                println!("send message result: {:?}", &result);
                ()
            })
            .map_err(|e| {
                eprintln!("Could not send message: {:?}", e);
                format!("send_message error: {:?}", e)
            })
    })
}

fn create_queue(
    client: Arc<RusotoSqsClient>,
    queue_name: String,
) -> impl Future<Item = String, Error = ()> {
    println!("create_queue called!");
    let mut request = CreateQueueRequest::default();
    request.queue_name = queue_name;

    client
        .create_queue(request)
        .map(|result| {
            println!("create_queue result: {:?}", &result);
            result.queue_url.unwrap()
        })
        .map_err(|e| panic!("Could not create queue {:?}", e))
}

#[test]
#[ignore]
fn test_success() {
    println!("Beginning test_success");

    println!("Creating Channel");
    let (tx, rx) = mpsc::sync_channel::<Payload>(1);
    let payload = Payload::new("my message", Action::Success);

    let queue_name = "test-queue";
    let docker = clients::Cli::default();
    let node = docker.run(ElasticMQ::default());
    let host_port = node.get_host_port(9324).unwrap();
    let region = build_local_region(host_port);
    let config = Config::default().with_mode(Mode::AWS(region, queue_name.to_owned()));
    let sqs_client = build_sqs_client(host_port);

    let sqs_client_for_spawn = Arc::clone(&sqs_client);
    let payload_for_spawn = payload.clone();
    let mut rt = Runtime::new().unwrap();
    //    rt.spawn(lazy(move || {
    //        println!("Creating queue");
    //
    //        let sm_clone_sqs_client = Arc::clone(&sqs_client_for_spawn);
    //        let f = create_queue(Arc::clone(&sqs_client_for_spawn), queue_name.to_owned())
    //            .map(move |queue_url| {
    //                println!("spawning create message");
    //                tokio::spawn(send_message(sm_clone_sqs_client, queue_url.clone(), payload_for_spawn)
    //                    .map_err(|e| {
    //                        panic!("error sending message: {:?}", e)
    //                    }));
    //                queue_url
    //            })
    //            .map(move |queue_url| {
    //                println!("Queue successfully created: {:?}", &queue_url);
    //                let worker = TestWorker::new(tx);
    //                let processor = Processor::new(&config, Box::new(worker)).unwrap();
    //
    ////                println!("Before Spawn");
    //                tokio::spawn_async(processor.process());
    ////                println!("After spawn: {:?}", &spawn);
    ////                queue_url
    ////                    send_message(sm_clone_sqs_client, queue_url, payload_for_spawn)
    ////                        .map_err(|e| {
    ////                            panic!("error sending message: {:?}", e)
    ////                        })
    //                Ok(())
    //            });
    ////            .and_then(move |queue_url| {
    ////                send_message(sm_clone_sqs_client, queue_url, payload_for_spawn)
    ////                    .map_err(|e| {
    ////                        panic!("error sending message: {:?}", e)
    ////                    })
    ////            });
    //
    //        tokio::spawn(Box::new(f));
    //
    //        Ok(())
    //    }));

    let result = rx.recv_timeout(Duration::from_secs(30)).unwrap();

    let shutdown = rt.shutdown_now().wait();
    println!("Shutdown {:?}", &shutdown);

    assert_eq!(payload, result);
}
