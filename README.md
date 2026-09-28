# Question Timer

A minimal pure-Rust terminal app for tracking elapsed study time and completed questions. No HTML, no CSS, no JS — just `cargo run`.

## Run

Install Rust, then from this folder run:

```text
cargo run
```

## Controls

Type a key + Enter:

| Key | Action |
| --- | ------ |
| `enter` / `a` | +1 question |
| `u` | undo (-1) |
| `p` | pause / resume |
| `r` | reset timer (keeps count) |
| `c` | reset count (keeps timer) |
| `x` | reset everything |
| `q` | save + quit |

The screen also shows questions/hour and average time per question.

State is saved to disk (`$XDG_STATE_HOME/question_timer/state`, falling back to `~/.local/state/...`), so quitting and restarting resumes the session. While the timer is running, wall-clock time between sessions counts too.

## Project shape

Single file (`src/main.rs`), standard library only — zero dependencies.
