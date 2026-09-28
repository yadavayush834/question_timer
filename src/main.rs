//! question timer — pure-Rust terminal app.
//!
//! No HTML, no CSS, no JS, no web server, no external crates.

use std::io::{self, BufRead, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
struct State {
    elapsed: u64, // seconds
    count: u64,
    running: bool,
}

fn fmt_hms(total: u64) -> String {
    format!("{:02}:{:02}:{:02}", total / 3600, total % 3600 / 60, total % 60)
}

fn render(state: State) {
    let mut out = io::stdout().lock();
    let questions = if state.count == 1 {
        "question"
    } else {
        "questions"
    };
    let status = if state.running { "running" } else { "paused" };
    let _ = write!(
        out,
        "\x1B[2J\x1B[Hquestion timer\n\n  {}\n  {} {questions} · {status}\n\n  enter +1 · p pause · u undo · r time · c count · x all · q quit\n> ",
        fmt_hms(state.elapsed),
        state.count,
    );
    let _ = out.flush();
}

/// Returns `true` to quit.
fn apply(cmd: &str, state: &mut State) -> bool {
    match cmd.trim().to_lowercase().as_str() {
        "q" | "quit" | "exit" => true,
        "" | "a" | "+" | "add" => {
            state.count = state.count.saturating_add(1);
            false
        }
        "u" | "-" | "undo" => {
            state.count = state.count.saturating_sub(1);
            false
        }
        "p" | "pause" | "resume" => {
            state.running = !state.running;
            false
        }
        "r" | "reset" => {
            state.elapsed = 0;
            false
        }
        "c" => {
            state.count = 0;
            false
        }
        "x" | "clear" => {
            state.elapsed = 0;
            state.count = 0;
            false
        }
        _ => false,
    }
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

    let mut state = State {
        elapsed: 0,
        count: 0,
        running: true,
    };
    render(state);
    loop {
        thread::sleep(Duration::from_secs(1));
        for cmd in rx.try_iter() {
            if apply(&cmd, &mut state) {
                return;
            }
        }
        if state.running {
            state.elapsed = state.elapsed.saturating_add(1);
        }
        render(state);
    }
}
