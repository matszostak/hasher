use chrono::{DateTime, Utc};
use clap::ValueEnum;
use is_elevated::is_elevated;
use std::time::SystemTime;

lazy_static! {
    pub static ref DEVICE_TYPE: String = whoami::distro().unwrap();
}

pub enum WriterMsg {
    Hash(String),
    Error(String),
    Log(String),
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Algorithm {
    Md5,
    Sha1,
    Sha256,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum CSVSeparator {
    Comma,
    Spaces,
    Pipe,
}

#[derive(Clone, Debug)]
pub struct RunTimeEnv {
    pub timestamp: String,
    pub device_type: String,
    pub run_as_admin: bool,
}

pub fn convert_time_iso8601(time: SystemTime) -> String {
    let now: DateTime<Utc> = time.into();
    now.to_rfc3339()
}

impl Default for RunTimeEnv {
    fn default() -> RunTimeEnv {
        RunTimeEnv {
            timestamp: convert_time_iso8601(SystemTime::now()),
            device_type: DEVICE_TYPE.to_string(),
            run_as_admin: is_elevated(),
        }
    }
}
