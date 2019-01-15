#[macro_use]
extern crate serde_derive;

//#[macro_use]
//extern crate log;

mod errors;
mod model;
mod sqs;

//use tokio::io;
//use tokio::net::TcpStream;
//use tokio::prelude::*;

//use rusoto_core::Region;

fn main() {}

//#[derive(Debug, Clone)]
//pub struct SqsPollTask {
//    sqs: Arc<SqsClient>
//}
//
//impl SqsPollTask {
//    pub fn new(sqs: Arc<SqsClient>) -> Self {
//        SqsPollTask { sqs }
//    }
//
//    fn poll_sqs(&self) -> Async<> {
//        self.sqs.
//
//    }
//
//}
//
//impl Future for SqsPollTask {
//    type Item = ();
//    type Error = ();
//
//
//}
//

//fn determine_region() -> Region {
//    Region::Custom {
//        name: "Custom".to_string(),
//        endpoint: "http://localhost:9324".to_string(),
//    }
//}
