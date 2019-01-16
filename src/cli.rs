use crate::errors::ProcessorError::{self, CommandLineError};
use clap::{App, Arg, ArgMatches, SubCommand};
use rusoto_core::Region;
use std::str::FromStr;

pub type Queue = String;
pub type Port = u32;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Local(Port, Queue),
    AWS(Region, Queue),
}

#[derive(Debug)]
pub struct Cli {
    maybe_local: Option<String>,
    maybe_region: Option<String>,
    maybe_queue: Option<String>,
}

impl Cli {
    pub fn new() -> Self {
        let matches = get_matches();
        Cli {
            maybe_local: matches.value_of("local").map(|s| s.to_owned()),
            maybe_region: matches.value_of("region").map(|s| s.to_owned()),
            maybe_queue: matches.value_of("queue").map(|s| s.to_owned()),
        }
    }

    #[cfg(test)]
    fn new_with(
        maybe_local: Option<String>,
        maybe_region: Option<String>,
        maybe_queue: Option<String>,
    ) -> Self {
        Cli {
            maybe_local,
            maybe_region,
            maybe_queue,
        }
    }

    pub fn determine_mode(&self) -> Result<Mode, ProcessorError> {
        if let Some(queue) = self.maybe_queue.clone() {
            if let Some(port_string) = self.maybe_local.clone() {
                port_string
                    .parse::<u32>()
                    .map_err(|_| CommandLineError("Invalid Port"))
                    .map(|port| (queue, port))
                    .map(|(queue, port)| Mode::Local(port, queue.to_owned()))
            } else if let Some(region_string) = self.maybe_region.clone() {
                Region::from_str(region_string.as_ref())
                    .map_err(|_| CommandLineError("Invalid region specified"))
                    .map(|region| Mode::AWS(region, queue.to_owned()))
            } else {
                Err(CommandLineError(
                    "No local or region parameter was specified",
                ))
            }
        } else {
            Err(CommandLineError("No queue was specified"))
        }
    }
}

fn get_matches<'a>() -> ArgMatches<'a> {
    App::new("rs-queue-processor")
        .version("0.1")
        .author("Jack Wright")
        .about("Processes messages off of an SQS queue")
        .arg(
            Arg::with_name("local")
                .short("l")
                .long("local")
                .help("Run against a local Elastic MQ server running on port")
                .value_name("PORT")
                .takes_value(true)
                .conflicts_with("region")
                .required_unless_one(&["region"]),
        )
        .arg(
            Arg::with_name("region")
                .short("r")
                .long("region")
                .help("The Amazon region of the sqs server")
                .value_name("REGION")
                .takes_value(true)
                .conflicts_with("local")
                .required_unless_one(&["local"]),
        )
        .arg(
            Arg::with_name("queue")
                .short("q")
                .long("queue")
                .help("The name of the queue")
                .value_name("QUEUE")
                .takes_value(true)
                .required(true),
        )
        .get_matches()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_queue() {
        let cli = Cli::new_with(Some("23".to_owned()), Some("uswest2".to_owned()), None);
        assert!(cli.determine_mode().is_err())
    }

    #[test]
    fn test_no_local_or_region() {
        let cli = Cli::new_with(None, None, Some("foo".to_owned()));
        assert!(cli.determine_mode().is_err())
    }

    #[test]
    fn test_bad_local_port() {
        let cli = Cli::new_with(Some("sdf".to_owned()), None, Some("foo".to_owned()));
        assert!(cli.determine_mode().is_err())
    }

    #[test]
    fn test_bad_region() {
        let cli = Cli::new_with(None, Some("usswest2".to_owned()), Some("foo".to_owned()));
        assert!(cli.determine_mode().is_err())
    }

    #[test]
    fn test_good_local() {
        let cli = Cli::new_with(
            Some("23".to_owned()),
            Some("uswest2".to_owned()),
            Some("foo".to_owned()),
        );
        assert_eq!(
            Mode::Local(23, "foo".to_owned()),
            cli.determine_mode().unwrap()
        )
    }

    #[test]
    fn test_good_aws() {
        let cli = Cli::new_with(None, Some("uswest2".to_owned()), Some("foo".to_owned()));
        assert_eq!(
            Mode::AWS(Region::UsWest2, "foo".to_owned()),
            cli.determine_mode().unwrap()
        )
    }

}
