use clap::{ValueEnum};
use chrono::{DateTime, Utc};
use is_elevated::is_elevated;use std::time::SystemTime;
use std::io::{self};

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


pub fn convert_time_iso8601(time: SystemTime) -> io::Result<String> {
    let now: DateTime<Utc> = time.into();
    Ok(now.to_rfc3339())
}

impl Default for RunTimeEnv {
    fn default() -> RunTimeEnv {
        RunTimeEnv {
            timestamp: convert_time_iso8601(SystemTime::now()).unwrap_or("1970-01-01T02:00:00+02:00Z".to_owned()),
            device_type: DEVICE_TYPE.to_string(),
            run_as_admin: is_elevated(),
        }
    }
}