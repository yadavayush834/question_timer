//! question timer — pure-Rust terminal app.
//!
//! No HTML, no CSS, no JS, no web server, no external crates.

use std::io::{self, BufRead, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn fmt_hms(total: u64) -> String {
    format!("{:02}:{:02}:{:02}", total / 3600, total % 3600 / 60, total % 60)
}

fn render(elapsed: u64) {
    let mut out = io::stdout().lock();
    let _ = write!(
        out,
        "\x1B[2J\x1B[Hquestion timer\n\n  {}\n\n  q quit\n> ",
        fmt_hms(elapsed),
    );
    let _ = out.flush();
}

fn main() {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(cmd) => {
                    if tx.send(cmd).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut elapsed: u64 = 0;
    render(elapsed);
    loop {
        thread::sleep(Duration::from_secs(1));
        for cmd in rx.try_iter() {
            if cmd.trim().to_lowercase() == "q" {
                return;
            }
        }
        elapsed = elapsed.saturating_add(1);
        render(elapsed);
    }
}
