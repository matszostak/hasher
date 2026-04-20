use clap::Parser;
use crate::helpers::{Algorithm, CSVSeparator};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, term_width = 120, max_term_width = 200)]
pub(crate) struct Args {
    /// Algorithm(s) to use. Provide multiple values to use multiple algorithms
    #[arg(short, long, num_args = 1..4)] // 4 Algorithms
    pub algorithm: Vec<Algorithm>,

    /// Target directory
    #[arg(short, long)]
    pub target: String,

    /// Output file
    #[arg(short, long = "out_path", default_value = "hashes.txt")]
    pub out_path: String,

    /// Number of worker threads
    #[arg(short, long, default_value_t = 8)]
    pub count: u8,

    /// Creates a log file if specified
    #[arg(short, long = "log")]
    pub log_path: Option<String>,

    /// CSV separator for the output file
    #[arg(short='s', long="csv_separator", value_enum, default_value_t = CSVSeparator::Spaces)]
    pub csv_separator: CSVSeparator,

    /// Skip CSV header in output file
    #[arg(long="skip_header", action = clap::ArgAction::SetTrue)]
    pub skip_header: bool,

    /// Include file metadata
    #[arg(long="metadata", action = clap::ArgAction::SetTrue)]
    pub metadata: bool,
}