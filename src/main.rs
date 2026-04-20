use clap::Parser;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, thread};
use std::{fs::OpenOptions, path::Path};
use walkdir::WalkDir;

#[macro_use]
extern crate lazy_static;
extern crate whoami;

mod cli;
mod hash;
mod helpers;

use cli::Args;
use helpers::{Algorithm, RunTimeEnv, WriterMsg, convert_time_iso8601};

fn main() {
    let args = Args::parse();
    let target_dir = args.target;
    let output_path = args.out_path;
    let hash_algorithm = args.algorithm;
    let csv_separator = args.csv_separator;
    let log_path = args.log_path;
    let skip_header = args.skip_header;
    let include_metadata = args.metadata;
    let skip_std_out = args.skip_std_out;

    if !Path::new(&target_dir).exists() {
        eprintln!("Target directory does not exist.");
        return;
    }

    let csv_separator_str = csv_separator.as_str();
    let hashes_str_vec = hash_algorithm
        .iter()
        .map(|algo| algo.as_str())
        .collect::<Vec<_>>()
        .join(csv_separator_str);
    let hashes_str_vec_clone = hashes_str_vec.clone();

    let (work_tx, work_rx) = unbounded::<PathBuf>();
    let (writer_tx, writer_rx) = unbounded::<WriterMsg>();

    let worker_count = args.count.into();

    let writer_handle = thread::spawn(move || {
        writer_loop(
            output_path,
            log_path,
            skip_header,
            include_metadata,
            csv_separator_str,
            hashes_str_vec,
            writer_rx,
        );
    });
    let env: RunTimeEnv = RunTimeEnv::default();
    writer_tx.send(WriterMsg::Log("START".to_string())).ok();
    writer_tx.send(WriterMsg::Log("Hasher".to_string())).ok();
    writer_tx
        .send(WriterMsg::Log(format!("Time: {:#?}", env.timestamp)))
        .ok();
    writer_tx
        .send(WriterMsg::Log(format!("Device: {:#?}", env.device_type)))
        .ok();
    writer_tx
        .send(WriterMsg::Log(format!("Elevated: {:#?}", env.run_as_admin)))
        .ok();
    writer_tx
        .send(WriterMsg::Log(format!("Algorithm: {hashes_str_vec_clone}")))
        .ok();
    writer_tx
        .send(WriterMsg::Log("Log START:".to_string()))
        .ok();

    // WORKERS
    let algos: Arc<[Algorithm]> = hash_algorithm.into();
    let mut handles = Vec::new();
    for _worker_id in 0..worker_count {
        let rx = work_rx.clone();
        let writer_tx = writer_tx.clone();
        let algos = Arc::clone(&algos);

        handles.push(thread::spawn(move || {
            worker_loop(
                rx,
                writer_tx,
                &algos,
                csv_separator_str,
                include_metadata,
                skip_std_out,
            );
        }));
    }

    for entry in WalkDir::new(target_dir)
        .follow_links(false)
        .same_file_system(false)
        .into_iter()
        .filter_map(|e| {
            if let Err(err) = &e {
                writer_tx.send(WriterMsg::Error(format!("{err}"))).ok();
            }
            e.ok()
        })
        .filter(|e| e.file_type().is_file())
    {
        work_tx.send(entry.path().to_path_buf()).unwrap();
    }

    drop(work_tx);

    for h in handles {
        h.join().unwrap();
    }
    writer_tx.send(WriterMsg::Log("END".to_string())).ok(); // TODO - add time of end
    drop(writer_tx);
    writer_handle.join().unwrap();
}

#[allow(clippy::needless_pass_by_value)]
fn writer_loop(
    output_path: String,
    log_path: Option<String>,
    skip_header: bool,
    include_metadata: bool,
    csv_separator_str: &str,
    hashes_str: String,
    writer_rx: Receiver<WriterMsg>,
) {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)
        .unwrap();

    let mut log_f = log_path.map(|log_path| {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .unwrap()
    });

    if !skip_header {
        if include_metadata {
            let metadata_headers = format!(
                "size{csv_separator_str}modified{csv_separator_str}accessed{csv_separator_str}created",
            );
            writeln!(
                f,
                "{hashes_str}{csv_separator_str}{metadata_headers}{csv_separator_str}path"
            )
            .ok();
        } else {
            writeln!(f, "{hashes_str}{csv_separator_str}path").ok();
        }
    }

    while let Ok(msg) = writer_rx.recv() {
        match msg {
            WriterMsg::Hash(line) => {
                writeln!(f, "{line}").ok();
                if let Some(ref mut file) = log_f {
                    writeln!(file, "HASH {line}").ok();
                }
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
}

#[allow(clippy::needless_pass_by_value)]
fn worker_loop(
    rx: Receiver<PathBuf>,
    writer_tx: Sender<WriterMsg>,
    hash_algorithm: &Arc<[Algorithm]>,
    csv_separator: &str,
    include_metadata: bool,
    skip_std_out: bool,
) {
    for path in rx {
        let mut hashes = match hash::hash_file(&path, hash_algorithm) {
            Ok((file_hashes, _bytes)) => file_hashes,
            Err(e) => {
                writer_tx
                    .send(WriterMsg::Error(format!(
                        "{}{}{}",
                        path.display(),
                        csv_separator,
                        e
                    )))
                    .ok();
                continue;
            }
        };
        if include_metadata {
            let metadata = fs::metadata(&path);
            match metadata {
                Ok(meta) => {
                    hashes.push(meta.len().to_string());
                    hashes.push(convert_time_iso8601(meta.modified().unwrap()));
                    hashes.push(convert_time_iso8601(meta.accessed().unwrap()));
                    hashes.push(convert_time_iso8601(meta.created().unwrap()));
                }
                Err(e) => {
                    writer_tx
                        .send(WriterMsg::Error(format!(
                            "{}{}{}",
                            path.display(),
                            csv_separator,
                            e
                        )))
                        .ok();
                    continue;
                }
            }
        }

        let line = format!(
            "{}{}{}",
            hashes.join(csv_separator),
            csv_separator,
            path.display()
        );
        if !skip_std_out {
            println!("{}", &line);
        }
        writer_tx.send(WriterMsg::Hash(line)).ok();
    }
}
