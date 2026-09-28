use axum::{response::Html, routing::get, Router};
use std::net::SocketAddr;

const PAGE: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="theme-color" content="#111412">
  <meta name="description" content="A minimal persistent timer for tracking completed questions.">
  <title>Question Timer</title>
  <style>
    :root {
      color-scheme: dark;
      --canvas: #111412;
      --surface: #191d1a;
      --line: #343b36;
      --muted: #8f9a91;
      --text: #eef3ee;
      --accent: #b9e769;
      --accent-ink: #18200f;
    }

    * { box-sizing: border-box; }

    body {
      margin: 0;
      min-height: 100vh;
      display: grid;
      place-items: center;
      background: var(--canvas);
      color: var(--text);
      font-family: "IBM Plex Sans", "Segoe UI", sans-serif;
    }

    main {
      width: min(100% - 40px, 560px);
      padding: 28px 0 34px;
    }

    header {
      display: flex;
      justify-content: space-between;
      align-items: baseline;
      padding: 0 2px 18px;
      border-bottom: 1px solid var(--line);
    }

    h1 {
      margin: 0;
      color: var(--text);
      font-family: Georgia, "Times New Roman", serif;
      font-size: clamp(1.35rem, 4vw, 1.75rem);
      font-weight: 400;
      letter-spacing: 0;
    }

    .eyebrow {
      margin: 0;
      color: var(--muted);
      font-size: 0.7rem;
      font-weight: 600;
      letter-spacing: 0.12em;
      text-transform: uppercase;
    }

    .panel {
      margin-top: 18px;
      padding: clamp(28px, 7vw, 52px) clamp(22px, 7vw, 54px) 30px;
      border: 1px solid var(--line);
      background: var(--surface);
    }

    .label {
      margin: 0 0 12px;
      color: var(--muted);
      font-size: 0.75rem;
      letter-spacing: 0.1em;
      text-transform: uppercase;
    }

    .timer {
      margin: 0;
      color: var(--accent);
      font-family: "Courier New", monospace;
      font-size: clamp(3.2rem, 14vw, 6.8rem);
      font-weight: 400;
      letter-spacing: 0.02em;
      line-height: 0.96;
      font-variant-numeric: tabular-nums;
    }

    .timer-rule {
      height: 1px;
      margin: 28px 0 26px;
      background: var(--line);
    }

    .count-row {
      display: flex;
      align-items: end;
      justify-content: space-between;
      gap: 20px;
    }

    .count {
      margin: 0;
      color: var(--text);
      font-family: "Courier New", monospace;
      font-size: clamp(2.8rem, 10vw, 4.5rem);
      line-height: 1;
      font-variant-numeric: tabular-nums;
    }

    button {
      border: 1px solid var(--accent);
      border-radius: 0;
      padding: 15px 20px;
      background: var(--accent);
      color: var(--accent-ink);
      cursor: pointer;
      font: inherit;
      font-size: 0.85rem;
      font-weight: 700;
      letter-spacing: 0.04em;
      transition: background 140ms ease, color 140ms ease, transform 140ms ease;
    }

    button:hover { background: transparent; color: var(--accent); }
    button:active { transform: translateY(1px); }
    button:focus-visible { outline: 2px solid var(--text); outline-offset: 4px; }

    .footer {
      display: flex;
      justify-content: flex-end;
      margin-top: 22px;
    }

    .reset {
      border: 0;
      padding: 5px 0;
      background: transparent;
      color: var(--muted);
      font-size: 0.8rem;
      font-weight: 400;
      letter-spacing: 0;
    }

    .reset:hover { color: var(--text); background: transparent; }

    @media (max-width: 420px) {
      main { width: min(100% - 28px, 560px); }
      .count-row { align-items: stretch; flex-direction: column; }
      .count-row button { width: 100%; }
    }

    @media (prefers-reduced-motion: reduce) {
      *, *::before, *::after { transition-duration: 0.01ms !important; }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <h1>Question Timer</h1>
      <p class="eyebrow">Study session</p>
    </header>

    <section class="panel" aria-labelledby="timer-label">
      <p class="label" id="timer-label">Elapsed time</p>
      <p class="timer" id="timer" aria-live="polite">00:00:00</p>
      <div class="timer-rule" aria-hidden="true"></div>

      <div class="count-row">
        <div>
          <p class="label">Questions completed</p>
          <p class="count" id="count" aria-live="polite">0</p>
        </div>
        <button id="increment" type="button">Add question</button>
      </div>

      <div class="footer">
        <button class="reset" id="reset" type="button">Reset timer</button>
      </div>
    </section>
  </main>

  <script>
    (() => {
      const storageKey = "question-timer-state";
      const timerElement = document.getElementById("timer");
      const countElement = document.getElementById("count");
      const incrementButton = document.getElementById("increment");
      const resetButton = document.getElementById("reset");

      const readState = () => {
        try {
          const saved = JSON.parse(localStorage.getItem(storageKey));
          if (saved && Number.isFinite(saved.elapsed) && Number.isFinite(saved.count)) {
            return { elapsed: Math.max(0, saved.elapsed), count: Math.max(0, Math.floor(saved.count)), savedAt: saved.savedAt };
          }
        } catch (_) { /* Start fresh when storage is unavailable or invalid. */ }
        return { elapsed: 0, count: 0, savedAt: Date.now() };
      };

      let state = readState();
      if (Number.isFinite(state.savedAt)) {
        state.elapsed += Math.max(0, Math.floor((Date.now() - state.savedAt) / 1000));
      }
      let lastTickAt = Date.now();

      const save = () => {
        try {
          localStorage.setItem(storageKey, JSON.stringify({
            elapsed: state.elapsed,
            count: state.count,
            savedAt: Date.now()
          }));
        } catch (_) { /* The timer remains usable without browser storage. */ }
      };

      const formatTime = (totalSeconds) => {
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor((totalSeconds % 3600) / 60);
        const seconds = totalSeconds % 60;
        return [hours, minutes, seconds].map((part) => String(part).padStart(2, "0")).join(":");
      };

      const render = () => {
        timerElement.textContent = formatTime(state.elapsed);
        countElement.textContent = String(state.count);
      };

      incrementButton.addEventListener("click", () => {
        state.count += 1;
        render();
        save();
      });

      resetButton.addEventListener("click", () => {
        state.elapsed = 0;
        lastTickAt = Date.now();
        render();
        save();
      });

      render();
      window.setInterval(() => {
        const now = Date.now();
        state.elapsed += Math.max(0, Math.floor((now - lastTickAt) / 1000));
        lastTickAt = now;
        render();
        save();
      }, 1000);
    })();
  </script>
</body>
</html>"##;

async fn index() -> Html<&'static str> {
    Html(PAGE)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(index));
    let address = SocketAddr::from(([127, 0, 0, 1], 3000));

    println!("Question Timer running at http://{address}");

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind server address");

    axum::serve(listener, app)
        .await
        .expect("server stopped unexpectedly");
}
