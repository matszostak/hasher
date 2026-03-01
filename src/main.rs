use clap::{Parser, ValueEnum};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::io::{Write};
use std::path::PathBuf;
use std::thread;
use std::{fs::OpenOptions, path::Path};
use walkdir::WalkDir;

mod hash;


enum WriterMsg {
    Hash(String),
    Error(String),
    Log(String),
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Algorithm {
    Md5,
    Sha1,
    Sha256,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum CSVSeparator {
    Comma,
    Spaces,
    Pipe,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, term_width = 120, max_term_width = 200)]

struct Args {
    /// Algorithm(s) to use. Provide multiple values to use multiple algorithms
    #[arg(short, long, num_args = 1..4)] // 4 Algorithms
    algorithm: Vec<Algorithm>,

    /// Target directory
    #[arg(short, long)]
    target: String,

    /// Output file
    #[arg(short, long, default_value = "hashes.txt")]
    out_path: String,

    /// Number of worker threads
    #[arg(short, long, default_value_t = 8)]
    count: u8,

    /// Creates a log file if specified
    #[arg(short, long = "log")]
    log_path: Option<String>,

    /// CSV separator for the output file
    #[arg(short='s', long="csv_separator", value_enum, default_value_t = CSVSeparator::Spaces)]
    csv_separator: CSVSeparator,

    /// Skip CSV header in output file
    #[arg(long="skip_header", action = clap::ArgAction::SetTrue)]
    skip_header: bool,

    /// Use experimental UI - work in progress
    #[arg(long="experimentalui", action = clap::ArgAction::SetTrue)]
    experimental_ui: bool,
}

fn main() {
    let args = Args::parse();
    let target_dir = args.target;
    let output_path = args.out_path;
    let hash_algorithm: Vec<Algorithm> = args.algorithm;
    let csv_separator = args.csv_separator;
    let log_path = args.log_path;
    let skip_header = args.skip_header;
    let _experimental_ui = args.experimental_ui;

    if !Path::new(&target_dir).exists() {
        eprintln!("Target directory does not exist.");
        return;
    }

    let csv_separator_str: &str;
    match csv_separator {
        CSVSeparator::Comma => csv_separator_str = ",",
        CSVSeparator::Spaces => csv_separator_str = "   ",
        CSVSeparator::Pipe => csv_separator_str = "|",
    }
    let hashes_str_vec = hash_algorithm
        .iter()
        .map(|a| format!("{:?}", a))
        .collect::<Vec<_>>()
        .join(csv_separator_str);
    let hashes_str_vec_clone = hashes_str_vec.clone();

    let (work_tx, work_rx) = unbounded::<PathBuf>();
    let (writer_tx, writer_rx) = unbounded::<WriterMsg>();

    let worker_count = args.count.into();

    let writer_handle = thread::spawn(move || {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(output_path)
            .unwrap();

        let mut log_f: Option<std::fs::File> = None;
        if log_path.is_some() {
            log_f = Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_path.unwrap())
                    .unwrap(),
            );
        }

        if !skip_header {
            writeln!(
                f,
                "{}",
                format!("{}{}{}", hashes_str_vec, csv_separator_str, "path")
            )
            .ok();
        }

        while let Ok(msg) = writer_rx.recv() {
            match msg {
                WriterMsg::Hash(line) => {
                    writeln!(f, "{line}").ok();
                }
                WriterMsg::Error(line) => {
                    if let Some(ref mut file) = log_f {
                        writeln!(file, "ERR {line}").ok();
                    }
                }
                WriterMsg::Log(line) => {
                    if let Some(ref mut file) = log_f {
                        writeln!(file, "LOG {line}").ok();
                    }
                }
            }
        }
    });

    writer_tx.send(WriterMsg::Log(format!("START"))).ok(); // TODO - add time of start
    writer_tx.send(WriterMsg::Log(format!("Hasher"))).ok();
    writer_tx.send(WriterMsg::Log(format!("Options:"))).ok();
    writer_tx
        .send(WriterMsg::Log(format!(
            "Algorithm: {}",
            hashes_str_vec_clone
        )))
        .ok();

    // WORKERS
    let mut handles = Vec::new();
    for _worker_id in 0..worker_count {
        let rx = work_rx.clone();
        let writer_tx = writer_tx.clone();
        let algo = hash_algorithm.clone();

        handles.push(thread::spawn(move || {
            worker_loop(rx, writer_tx, algo, csv_separator_str)
        }));
    }

    // WALK FILES
    for entry in WalkDir::new(target_dir)
        .follow_links(false)
        .same_file_system(false)
        .into_iter()

    {
        match entry {
            Ok(entry) => {
                work_tx.send(entry.path().to_path_buf()).unwrap();
            }
            Err(err) => {
                writer_tx.send(WriterMsg::Error(format!("{}", err))).ok();
            }
        }
    }

    drop(work_tx);

    for h in handles {
        h.join().unwrap();
    }
    writer_tx.send(WriterMsg::Log(format!("END"))).ok(); // TODO - add time of end
    drop(writer_tx);
    writer_handle.join().unwrap();
}

fn worker_loop(
    rx: Receiver<PathBuf>, 
    writer_tx: Sender<WriterMsg>,
    hash_algorithm: Vec<Algorithm>,
    csv_separator: &str,
) {
    for path in rx.iter() {
        let mut hashes = Vec::with_capacity(hash_algorithm.len());
        let mut error_occurred = false;

        for algo in &hash_algorithm {
            let result = match algo {
                Algorithm::Md5 => hash::hash_file_md5(&path),
                Algorithm::Sha1 => hash::hash_file_sha1(&path),
                Algorithm::Sha256 => hash::hash_file_sha256(&path),
            };

            match result {
                Ok((hash, _bytes)) => {
                    hashes.push(hash);
                }
                Err(e) => {
                    error_occurred = true;

                    writer_tx
                        .send(WriterMsg::Error(format!(
                            "{}{}{}",
                            path.display(),
                            csv_separator,
                            e
                        )))
                        .ok();

                    break;
                }
            }
        }

        if !error_occurred {
            let line = format!(
                "{}{}{}",
                hashes.join(csv_separator),
                csv_separator,
                path.display()
            );
            
            writer_tx.send(WriterMsg::Hash(line.clone())).ok();
            println!("{}", line);
        }
    }
}