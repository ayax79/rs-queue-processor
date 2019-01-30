# rs-queue-processor
A library to process messages from an Amazon SQS queue. 

To use this library simply implement the Worker trait:

```rust
use rs_queue_processor::worker::{Worker, WorkerFuture}
use futures::future::lazy;


#[derive(Clone, Default)]
pub struct WorkerImpl;

impl Worker for WorkerImpl {
    fn process(&self, m: SqsMessage) -> Box<WorkerFuture> {
      Box::new(lazy(|| {
        println!("Received message: {:#?}", m);
      }))    
    }
}
```

Then initialize the processor:

```rust
use rs_queue_processor::config::Cli;
use rs_queue_processor::processor::Processor;

fn main() {
    match Cli::new().build_config() {
        Ok(config) => {
            let worker = WorkerImpl::default();
            let processor = Processor::new(&config, Box::new(worker));
            tokio::run(processor.unwrap().process());
        }
        Err(e) => {
            panic!("{}", e);
        }
    }
}
```

