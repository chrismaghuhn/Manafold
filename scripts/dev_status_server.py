#!/usr/bin/env python3
"""Development status server for the Manafold project.

Serves a dashboard on port 3000 showing build and test results.
The build runs in a background thread; the page auto-refreshes until complete.
"""

from __future__ import annotations

import html
import json
import os
import subprocess
import threading
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PORT = int(os.environ.get("PORT", "3000"))
CARGO_HOME = os.environ.get("CARGO_HOME", os.path.expanduser("~/.cargo"))
VENV_PYTHON = str(ROOT / ".venv" / "bin" / "python")

STEPS = [
    ("bootstrap", "Bootstrap Python venv", ["python3", "scripts/bootstrap.py"]),
    (
        "fast_checks",
        "Python fast checks",
        [VENV_PYTHON, "scripts/run_checks.py", "fast", "--allow-missing-tools"],
    ),
    (
        "cargo_check",
        "Rust workspace check",
        ["cargo", "check", "--workspace", "--all-targets", "--all-features", "--locked"],
    ),
    (
        "cargo_test",
        "Rust workspace tests",
        ["cargo", "test", "--workspace", "--all-features", "--locked"],
    ),
]


class BuildRunner:
    def __init__(self) -> None:
        self.results: dict[str, dict] = {}
        self.status = "idle"
        self.started_at: float | None = None
        self.finished_at: float | None = None
        self.lock = threading.Lock()

    def run(self) -> None:
        with self.lock:
            self.status = "running"
            self.started_at = time.time()
            self.finished_at = None
            self.results = {}

        env = os.environ.copy()
        cargo_bin = os.path.join(CARGO_HOME, "bin")
        env["PATH"] = f"{cargo_bin}:{env.get('PATH', '')}"

        all_passed = True
        for step_id, step_name, command in STEPS:
            with self.lock:
                self.results[step_id] = {
                    "name": step_name,
                    "status": "running",
                    "output": "",
                    "elapsed": None,
                }

            started = time.perf_counter()
            try:
                result = subprocess.run(
                    command,
                    cwd=ROOT,
                    env=env,
                    capture_output=True,
                    text=True,
                    timeout=600,
                )
                elapsed = time.perf_counter() - started
                step_status = "passed" if result.returncode == 0 else "failed"
                if step_status == "failed":
                    all_passed = False
                output = (result.stdout or "") + (result.stderr or "")
                if len(output) > 8000:
                    output = output[:8000] + "\n... (truncated)"
            except subprocess.TimeoutExpired:
                elapsed = time.perf_counter() - started
                step_status = "failed"
                all_passed = False
                output = f"TIMEOUT after {elapsed:.1f}s"
            except Exception as exc:
                elapsed = time.perf_counter() - started
                step_status = "failed"
                all_passed = False
                output = str(exc)

            with self.lock:
                self.results[step_id] = {
                    "name": step_name,
                    "status": step_status,
                    "output": output,
                    "elapsed": round(elapsed, 2),
                }

        with self.lock:
            self.status = "passed" if all_passed else "failed"
            self.finished_at = time.time()

    def get_state(self) -> dict:
        with self.lock:
            return {
                "status": self.status,
                "results": {k: dict(v) for k, v in self.results.items()},
                "started_at": self.started_at,
                "finished_at": self.finished_at,
            }


runner = BuildRunner()

HTML_PAGE = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Manafold — Dev Status</title>
<style>
  :root {
    --bg: #0d1117;
    --card: #161b22;
    --border: #30363d;
    --text: #c9d1d9;
    --muted: #8b949e;
    --green: #3fb950;
    --red: #f85149;
    --yellow: #d29922;
    --blue: #58a6ff;
    --radius: 8px;
  }
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    background: var(--bg);
    color: var(--text);
    line-height: 1.6;
    padding: 24px;
    max-width: 960px;
    margin: 0 auto;
  }
  header { margin-bottom: 24px; }
  h1 { font-size: 1.8rem; font-weight: 600; }
  header p { color: var(--muted); font-size: 0.95rem; margin-top: 4px; }
  .meta {
    display: flex; gap: 16px; margin-top: 8px;
    font-size: 0.85rem; color: var(--muted);
  }
  .meta span { display: flex; align-items: center; gap: 4px; }
  .summary {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 20px;
    margin-bottom: 20px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .summary-status {
    font-size: 1.3rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .dot {
    width: 12px; height: 12px;
    border-radius: 50%;
    display: inline-block;
  }
  .dot.passed { background: var(--green); }
  .dot.failed { background: var(--red); }
  .dot.running { background: var(--yellow); animation: pulse 1.5s infinite; }
  .dot.idle { background: var(--muted); }
  @keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.4; } }
  .summary-meta { color: var(--muted); font-size: 0.9rem; }
  .steps { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  @media (max-width: 700px) { .steps { grid-template-columns: 1fr; } }
  .step {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
  }
  .step-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .step-name { font-weight: 600; font-size: 0.95rem; }
  .step-badge {
    font-size: 0.75rem;
    font-weight: 600;
    padding: 3px 10px;
    border-radius: 20px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .badge-passed { background: rgba(63,185,80,0.15); color: var(--green); }
  .badge-failed { background: rgba(248,81,73,0.15); color: var(--red); }
  .badge-running { background: rgba(210,153,34,0.15); color: var(--yellow); }
  .badge-pending { background: rgba(139,148,158,0.15); color: var(--muted); }
  .step-elapsed { font-size: 0.8rem; color: var(--muted); margin-bottom: 8px; }
  .step-output {
    background: #010409;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 10px;
    font-family: "SF Mono", Monaco, Consolas, monospace;
    font-size: 0.8rem;
    color: var(--muted);
    max-height: 240px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    display: none;
  }
  .step-output.visible { display: block; }
  .toggle-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.75rem;
    padding: 3px 10px;
    border-radius: 4px;
    cursor: pointer;
  }
  .toggle-btn:hover { border-color: var(--blue); color: var(--blue); }
  .actions { margin-top: 20px; }
  .btn {
    background: var(--blue);
    color: #fff;
    border: none;
    padding: 10px 20px;
    border-radius: var(--radius);
    font-size: 0.9rem;
    cursor: pointer;
    font-weight: 500;
  }
  .btn:hover { opacity: 0.9; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  footer { margin-top: 24px; color: var(--muted); font-size: 0.8rem; }
  a { color: var(--blue); text-decoration: none; }
  a:hover { text-decoration: underline; }
</style>
</head>
<body>
<header>
  <h1>Manafold</h1>
  <p>Headless, deterministic, ML-native Magic: The Gathering rules and simulation engine</p>
  <div class="meta">
    <span>v0.2.2</span>
    <span>·</span>
    <span>Rust 1.85.1</span>
    <span>·</span>
    <span>Python 3.13.15</span>
    <span>·</span>
    <span>14 workspace crates</span>
  </div>
</header>

<div id="summary" class="summary">
  <div class="summary-status">
    <span class="dot idle"></span>
    <span id="summary-text">Initializing…</span>
  </div>
  <div class="summary-meta" id="summary-meta"></div>
</div>

<div id="steps" class="steps"></div>

<div class="actions">
  <button class="btn" id="recheck-btn" onclick="recheck()">Re-run checks</button>
</div>

<footer>
  <a href="https://github.com/chrismaghuhn/Manafold" target="_blank">GitHub</a>
  · <a href="/README.md">README</a>
  · Dev status server on port 3000
</footer>

<script>
const STEP_DEFS = [
  { id: "bootstrap", name: "Bootstrap Python venv" },
  { id: "fast_checks", name: "Python fast checks" },
  { id: "cargo_check", name: "Rust workspace check" },
  { id: "cargo_test", name: "Rust workspace tests" },
];

function esc(s) {
  const d = document.createElement("div");
  d.textContent = s || "";
  return d.innerHTML;
}

function renderSteps(state) {
  const container = document.getElementById("steps");
  container.innerHTML = "";
  for (const def of STEP_DEFS) {
    const r = state.results[def.id];
    const status = r ? r.status : "pending";
    const elapsed = r && r.elapsed != null ? r.elapsed.toFixed(2) + "s" : "";
    const output = r ? r.output : "";

    const card = document.createElement("div");
    card.className = "step";

    const header = document.createElement("div");
    header.className = "step-header";

    const name = document.createElement("span");
    name.className = "step-name";
    name.textContent = def.name;

    const badge = document.createElement("span");
    badge.className = "step-badge badge-" + status;
    badge.textContent = status;

    header.appendChild(name);
    header.appendChild(badge);

    let outBtn = null;
    if (output) {
      outBtn = document.createElement("button");
      outBtn.className = "toggle-btn";
      outBtn.textContent = "Show output";
      outBtn.onclick = function () {
        const out = card.querySelector(".step-output");
        out.classList.toggle("visible");
        outBtn.textContent = out.classList.contains("visible") ? "Hide output" : "Show output";
      };
      header.appendChild(outBtn);
    }

    const el = document.createElement("div");
    el.className = "step-elapsed";
    el.textContent = elapsed;

    card.appendChild(header);
    card.appendChild(el);

    if (output) {
      const pre = document.createElement("div");
      pre.className = "step-output";
      pre.textContent = output;
      card.appendChild(pre);
    }

    container.appendChild(card);
  }
}

function renderSummary(state) {
  const dot = document.querySelector(".dot");
  const text = document.getElementById("summary-text");
  const meta = document.getElementById("summary-meta");

  dot.className = "dot " + state.status;

  const labels = {
    idle: "Idle",
    running: "Building…",
    passed: "All checks passed",
    failed: "Some checks failed",
  };
  text.textContent = labels[state.status] || state.status;

  const passed = Object.values(state.results).filter(r => r.status === "passed").length;
  const failed = Object.values(state.results).filter(r => r.status === "failed").length;
  const running = Object.values(state.results).filter(r => r.status === "running").length;
  const total = STEP_DEFS.length;

  let parts = [`${passed}/${total} steps passed`];
  if (failed > 0) parts.push(`${failed} failed`);
  if (running > 0) parts.push(`${running} running`);

  if (state.started_at) {
    const end = state.finished_at || Date.now() / 1000;
    parts.push(`${(end - state.started_at).toFixed(1)}s elapsed`);
  }

  meta.textContent = parts.join(" · ");

  const btn = document.getElementById("recheck-btn");
  btn.disabled = state.status === "running";
}

let pollTimer = null;

async function fetchStatus() {
  try {
    const res = await fetch("/api/status");
    const state = await res.json();
    renderSummary(state);
    renderSteps(state);
    if (state.status === "running") {
      pollTimer = setTimeout(fetchStatus, 3000);
    }
  } catch (e) {
    pollTimer = setTimeout(fetchStatus, 3000);
  }
}

function recheck() {
  fetch("/recheck").then(() => {
    if (pollTimer) clearTimeout(pollTimer);
    setTimeout(fetchStatus, 500);
  });
}

fetchStatus();
</script>
</body>
</html>"""


class Handler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:
        if self.path == "/api/status":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            self.wfile.write(json.dumps(runner.get_state()).encode())
        elif self.path in ("/", "/index.html"):
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            self.wfile.write(HTML_PAGE.encode())
        elif self.path == "/recheck":
            if runner.status != "running":
                threading.Thread(target=runner.run, daemon=True).start()
            self.send_response(302)
            self.send_header("Location", "/")
            self.end_headers()
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format: str, *args) -> None:
        pass


def main() -> None:
    threading.Thread(target=runner.run, daemon=True).start()
    server = HTTPServer(("0.0.0.0", PORT), Handler)
    print(f"Manafold dev status server on http://0.0.0.0:{PORT}", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
