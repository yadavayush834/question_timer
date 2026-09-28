//! question timer — pure-Rust terminal app.
//!
//! No HTML, no CSS, no JS, no web server, no external crates.

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy)]
struct State {
    elapsed: u64, // seconds
    count: u64,
    running: bool,
}

impl State {
    fn rate_per_hour(&self) -> Option<f64> {
        if self.elapsed > 0 && self.count > 0 {
            Some(self.count as f64 * 3600.0 / self.elapsed as f64)
        } else {
            None
        }
    }

    fn avg_per_question(&self) -> Option<u64> {
        if self.count > 0 {
            Some(self.elapsed / self.count)
        } else {
            None
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn state_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("question_timer").join("state");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("question_timer")
                .join("state");
        }
    }
    PathBuf::from(".question_timer_state")
}

/// On-disk format: `elapsed count running saved_at` (all integers).
fn load() -> State {
    let fallback = State {
        elapsed: 0,
        count: 0,
        running: true,
    };
    let raw = fs::read_to_string(state_path()).unwrap_or_default();
    let mut it = raw.split_whitespace();
    let elapsed: u64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let count: u64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let running: bool = it.next().map(|s| s == "1").unwrap_or(true);
    let saved_at: u64 = it.next().and_then(|s| s.parse().ok()).unwrap_or_else(now_unix);

    // Mirror the old localStorage behavior: time kept passing while away.
    let mut elapsed = elapsed;
    if running {
        elapsed = elapsed.saturating_add(now_unix().saturating_sub(saved_at));
    } else if raw.trim().is_empty() {
        return fallback;
    }
    State {
        elapsed,
        count,
        running,
    }
}

fn save(state: State) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
    let content = format!(
        "{} {} {} {}\n",
        state.elapsed,
        state.count,
        u8::from(state.running),
        now_unix()
    );
    let _ = fs::write(&path, content);
}

fn fmt_hms(total: u64) -> String {
    format!("{:02}:{:02}:{:02}", total / 3600, total % 3600 / 60, total % 60)
}

/// Compact duration: `48s`, `4m48s`, `1h05m`.
fn fmt_short(total: u64) -> String {
    if total < 60 {
        format!("{total}s")
    } else if total < 3600 {
        format!("{}m{:02}s", total / 60, total % 60)
    } else {
        format!("{}h{:02}m", total / 3600, total % 3600 / 60)
    }
}

fn render(state: State, notice: Option<&str>) {
    let mut out = io::stdout().lock();
    let questions = if state.count == 1 {
        "question"
    } else {
        "questions"
    };
    let status = if state.running { "running" } else { "paused" };
    let rate = state
        .rate_per_hour()
        .map(|r| format!("{r:.1}/hr"))
        .unwrap_or_else(|| "—".to_string());
    let avg = state
        .avg_per_question()
        .map(|a| format!("{}/q", fmt_short(a)))
        .unwrap_or_else(|| "—".to_string());
    let _ = write!(
        out,
        "\x1B[2J\x1B[Hquestion timer\n\n  {}\n  {} {questions} · {status}\n\n  {rate} · {avg}\n",
        fmt_hms(state.elapsed),
        state.count,
    );
    if let Some(msg) = notice {
        let _ = writeln!(out, "\n  ! {msg}");
    }
    let _ = write!(
        out,
        "\n  enter +1 · p pause · u undo · r time · c count · x all · q quit\n> ",
    );
    let _ = out.flush();
}

/// Returns `(quit, notice)`.
fn apply(cmd: &str, state: &mut State) -> (bool, Option<String>) {
    match cmd.trim().to_lowercase().as_str() {
        "q" | "quit" | "exit" => (true, None),
        "" | "a" | "+" | "add" => {
            state.count = state.count.saturating_add(1);
            (false, None)
        }
        "u" | "-" | "undo" => {
            state.count = state.count.saturating_sub(1);
            (false, None)
        }
        "p" | "pause" | "resume" => {
            state.running = !state.running;
            (false, None)
        }
        "r" | "reset" => {
            state.elapsed = 0;
            (false, None)
        }
        "c" => {
            state.count = 0;
            (false, None)
        }
        "x" | "clear" => {
            state.elapsed = 0;
            state.count = 0;
            (false, None)
        }
        "h" | "help" | "?" => (false, None),
        other => (false, Some(format!("unknown: \"{other}\""))),
    }
}

fn main() {
    let mut state = load();

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

    let mut last = Instant::now();
    let mut frac = Duration::ZERO;
    let mut last_rendered_sec = u64::MAX;
    let mut last_save = Instant::now();
    let mut notice: Option<(String, Instant)> = None;
    let mut dirty = true;

    loop {
        thread::sleep(Duration::from_millis(200));

        // Advance the clock by real elapsed time, keeping sub-second carry.
        let now = Instant::now();
        frac += now.duration_since(last);
        last = now;
        if state.running {
            while frac >= Duration::from_secs(1) {
                state.elapsed = state.elapsed.saturating_add(1);
                frac -= Duration::from_secs(1);
            }
        } else {
            frac = Duration::ZERO;
        }

        // Drain all pending commands.
        let mut quit = false;
        for cmd in rx.try_iter() {
            let (q, msg) = apply(&cmd, &mut state);
            match msg {
                Some(m) => notice = Some((m, Instant::now())),
                None => {
                    notice = None;
                    dirty = true;
                }
            }
            if q {
                quit = true;
                break;
            }
            // A time-affecting command restarts the sub-second carry
            // so the next tick is a full second away.
            frac = Duration::ZERO;
            dirty = true;
        }

        if quit {
            save(state);
            return;
        }

        // Expire transient notices.
        if let Some((_, at)) = &notice {
            if at.elapsed() > Duration::from_secs(4) {
                notice = None;
                dirty = true;
            }
        }

        if state.elapsed != last_rendered_sec || dirty {
            render(state, notice.as_ref().map(|(m, _)| m.as_str()));
            last_rendered_sec = state.elapsed;
            dirty = false;
        }
        if last_save.elapsed() > Duration::from_secs(5) {
            save(state);
            last_save = Instant::now();
        }
    }
}
