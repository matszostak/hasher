use std::{io::{Stdout, Write}, sync::{Arc, Mutex}, time::Duration};
use crossterm::{cursor::MoveTo, execute};

pub fn draw_dashboard(
    stdout: &Arc<Mutex<Stdout>>,
    workers: &[String],
    total_files: u64,
    elapsed: Duration,
) {
    let mut out = stdout.lock().unwrap();

    let secs = elapsed.as_secs_f64();

    execute!(out, MoveTo(0, 0)).unwrap();

    writeln!(out, "============ Hasher ============").unwrap();
    writeln!(out, "Files : {:<20}", total_files).unwrap();
    writeln!(out, "Time  : {:.1} s", secs).unwrap();
    // writeln!(out, "Rate  : {:.1} files/s", total_files as f64 / elapsed.as_secs_f64()).unwrap();
    writeln!(out, "================================").unwrap();

    //for (i, w) in workers.iter().enumerate() {
    //    println!("W{:02} | {}", i, w);
    //}
    const WORKER_LINE_WIDTH: usize = 120;
    // Worker lines
    for (i, w) in workers.iter().enumerate() {
        execute!(out, MoveTo(0, 5 + i as u16)).unwrap();
        let line = format!("W{:02} | {}", i, w);
        write!(out, "{:<width$}", line, width = WORKER_LINE_WIDTH).unwrap();
    }

    out.flush().unwrap();
}