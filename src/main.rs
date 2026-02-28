use clap::{Parser, ValueEnum};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use crossterm::{
    cursor::{Hide, MoveTo},
    execute,
    terminal::{Clear, ClearType},
};
use std::io::{Stdout, Write, stdout};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};
use walkdir::WalkDir;

mod dashboard;
mod hash;

enum UiMsg {
    WorkerStatus { worker_id: usize, text: String },
    FileDone,
}

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
#[command(version, about, long_about = None)]

struct Args {
    /// Algorithm to use //TODO: use multiple algorithms
    #[arg(short, long, value_enum, default_value_t = Algorithm::Md5)]
    algorithm: Algorithm,

    /// Target directory
    #[arg(short, long)]
    target: String,

    /// Output directory
    #[arg(short, long, default_value = "hashes.txt")]
    out: String,

    /// Number of worker threads
    #[arg(short, long, default_value_t = 8)]
    count: u8,

    /// Creates a log file if specified // TODO
    #[arg(short, long = "log", default_value = "none")]
    log_path: String,

    /// CSV separator for the output file
    #[arg(short='s', long="csv_separator", value_enum, default_value_t = CSVSeparator::Spaces)]
    csv_separator: CSVSeparator,
}

fn main() {
    let args = Args::parse();
    let target_dir = args.target;
    let output_path = args.out;
    let hash_algorithm = args.algorithm;
    let csv_separator = args.csv_separator;
    let log_path = args.log_path;

    if !Path::new(&target_dir).exists() {
        eprintln!("Target directory does not exist.");
        return;
    }

    let (work_tx, work_rx) = unbounded::<PathBuf>();
    let (writer_tx, writer_rx) = unbounded::<WriterMsg>();
    let (ui_tx, ui_rx) = unbounded::<UiMsg>();

    let output_file = Arc::new(Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(output_path.to_string())
            .unwrap(),
    ));

    // let worker_count = num_cpus::get();
    let worker_count = args.count.into();

    // UI THREAD
    let ui_handle = {
        let stdout = Arc::new(Mutex::new(stdout()));
        thread::spawn(move || ui_loop(ui_rx, worker_count, stdout))
    };

    let writer_handle = thread::spawn(move || {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(output_path)
            .unwrap();

        /* for line in writer_rx {
            writeln!(f, "{}", line).unwrap();
        } */

        let mut log_f: Option<std::fs::File> = None;
        if log_path != "none" {
            log_f = Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_path)
                    .unwrap(),
            );
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

    writer_tx.send(WriterMsg::Log(format!("START"))).ok();  // TODO - add time of start
    writer_tx.send(WriterMsg::Log(format!("Hasher"))).ok();
    writer_tx.send(WriterMsg::Log(format!("Options:"))).ok();
    writer_tx.send(WriterMsg::Log(format!("Algorithm: {:#?}", hash_algorithm))).ok();

    // WORKERS
    let mut handles = Vec::new();
    for worker_id in 0..worker_count {
        let rx = work_rx.clone();
        let ui_tx = ui_tx.clone();
        let writer_tx = writer_tx.clone();
        let file = output_file.clone();
        let algo = hash_algorithm;

        handles.push(thread::spawn(move || {
            worker_loop(rx, ui_tx, writer_tx, file, worker_id, algo, csv_separator)
        }));
    }

    // WALK FILES
    for entry in WalkDir::new(target_dir)
        .follow_links(false) // safer for forensics
        .same_file_system(false) // allow crossing mount points
        .into_iter()
    //.filter(|e| e.file_type().is_file())
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
    drop(ui_tx);
    ui_handle.join().unwrap();

    
}

fn worker_loop(
    rx: Receiver<PathBuf>,
    ui_tx: Sender<UiMsg>,
    writer_tx: Sender<WriterMsg>,
    output_file: Arc<Mutex<File>>,
    worker_id: usize,
    hash_algorithm: Algorithm,
    csv_separator: CSVSeparator,
) {
    let sep: &str;
    match csv_separator {
        CSVSeparator::Comma => sep = ",",
        CSVSeparator::Spaces => sep = "   ",
        CSVSeparator::Pipe => sep = "|",
    }
    for path in rx.iter() {
        let path_str = truncate_path(&path.display().to_string());

        ui_tx
            .send(UiMsg::WorkerStatus {
                worker_id,
                text: format!("HASH {}", path_str),
            })
            .ok();

        match hash_algorithm {
            Algorithm::Md5 => match hash::hash_file_md5(&path) {
                Ok((hash, _bytes)) => {
                    ui_tx.send(UiMsg::FileDone).ok();

                    let mut _f = output_file.lock().unwrap();
                    // writeln!(f, "{}  {}", hash, path.display()).ok();
                    writer_tx
                        .send(WriterMsg::Hash(format!(
                            "{}{}{}",
                            hash,
                            sep,
                            path.display()
                        )))
                        .ok();
                }
                Err(e) => {
                    /* ui_tx
                    .send(UiMsg::WorkerStatus {
                        worker_id,
                        text: format!("ERR  {}", e),
                    })
                    .ok(); */
                    writer_tx
                        .send(WriterMsg::Error(format!("{}{}{}", path.display(), sep, e)))
                        .ok();
                }
            },
            Algorithm::Sha1 => match hash::hash_file_sha1(&path) {
                Ok((hash, _bytes)) => {
                    ui_tx.send(UiMsg::FileDone).ok();

                    let mut _f = output_file.lock().unwrap();
                    writer_tx
                        .send(WriterMsg::Hash(format!(
                            "{}{}{}",
                            hash,
                            sep,
                            path.display()
                        )))
                        .ok();
                }
                Err(e) => {
                    /* ui_tx
                    .send(UiMsg::WorkerStatus {
                        worker_id,
                        text: format!("ERR  {}", e),
                    })
                    .ok(); */
                    writer_tx
                        .send(WriterMsg::Error(format!("{}{}{}", path.display(), sep, e)))
                        .ok();
                }
            },
            Algorithm::Sha256 => match hash::hash_file_sha256(&path) {
                Ok((hash, _bytes)) => {
                    ui_tx.send(UiMsg::FileDone).ok();

                    let mut _f = output_file.lock().unwrap();
                    writer_tx
                        .send(WriterMsg::Hash(format!(
                            "{}{}{}",
                            hash,
                            sep,
                            path.display()
                        )))
                        .ok();
                }
                Err(e) => {
                    /* ui_tx
                    .send(UiMsg::WorkerStatus {
                        worker_id,
                        text: format!("ERR  {}", e),
                    })
                    .ok(); */
                    writer_tx
                        .send(WriterMsg::Error(format!("{}{}{}", path.display(), sep, e)))
                        .ok();
                }
            },
        }
    }

    ui_tx
        .send(UiMsg::WorkerStatus {
            worker_id,
            text: "IDLE".into(),
        })
        .ok();
}

fn ui_loop(rx: Receiver<UiMsg>, workers: usize, stdout: Arc<Mutex<Stdout>>) {
    let mut worker_lines = vec![String::from("Waiting"); workers];
    let start = Instant::now();
    let mut total_files = 0u64;
    {
        let mut out = stdout.lock().unwrap();
        execute!(*out, Clear(ClearType::All), MoveTo(0, 0), Hide).unwrap();

        out.flush().unwrap();
        for _ in 0..(workers + 6) {
            writeln!(out).unwrap();
        }
    }

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(msg) => match msg {
                UiMsg::WorkerStatus { worker_id, text } => {
                    worker_lines[worker_id] = text;
                }
                UiMsg::FileDone => {
                    total_files += 1;
                }
            },

            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Normal — just redraw
            }

            Err(RecvTimeoutError::Disconnected) => {
                // All workers + main dropped sender
                break;
            }
        }

        dashboard::draw_dashboard(&stdout, &worker_lines, total_files, start.elapsed());
    }

    // Final draw
    dashboard::draw_dashboard(&stdout, &worker_lines, total_files, start.elapsed());
}

fn truncate_path(s: &str) -> String {
    const MAX: usize = 60;
    if s.len() > MAX {
        format!("...{}", &s[s.len() - MAX..])
    } else {
        s.to_string()
    }
}
