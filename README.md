# Question Timer

A minimal Rust website for tracking elapsed study time and completed questions.

## Run

Install Rust, then from this folder run:

```text
cargo run
```

Open `http://127.0.0.1:3000` in a browser.

To validate the project without starting the server, run `cargo check`.

The timer and question count are stored in browser `localStorage`, so refreshing the page keeps the session. `Reset timer` only resets elapsed time; the question count remains intact.

## Project shape

The server and page are written in Rust. The page is served directly from the binary, with no frontend framework or external assets.
