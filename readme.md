OKF Server (Rust)
A lightweight, fast, Rust‑powered Open Knowledge Format (OKF) server designed for agent frameworks such as Grok‑CLI and Helix.
This service hosts OKF bundles, tool schemas, and shared agent knowledge in a clean, versioned, network‑accessible format.

The goal is to provide a stable, low‑latency knowledge backend that both agents can consume without interfering with each other’s runtime.

Features
Serves OKF bundles over HTTP

Fast Rust backend using axum

YAML‑based bundle + tool definitions

Hot‑reloadable bundles (no restart required)

Clean separation between:

Grok‑CLI bundle

Helix bundle

Shared bundle

Simple REST API for tools and bundle metadata

Easy deployment on Proxmox (LXC or VM)

Project Structure
Code
okf/
├─ Cargo.toml
├─ README.md
├─ src/
│  ├─ main.rs
│  ├─ config.rs
│  ├─ server.rs
│  ├─ bundle.rs
│  ├─ model.rs
│  └─ error.rs
├─ bundles/
│  ├─ helix/
│  │  ├─ okf.yaml
│  │  └─ tools/
│  │     ├─ fs_read.yaml
│  │     └─ search.yaml
│  ├─ grok-cli/
│  │  ├─ okf.yaml
│  │  └─ tools/
│  │     └─ file_open.yaml
│  └─ shared/
│     ├─ okf.yaml
│     └─ tools/
│        └─ ping.yaml
├─ config/
│  └─ server.yaml
└─ py_okf/
   └─ reference Python OKF implementation
Configuration
`config/server.yaml` (override the path with `--config <path>`/`-c <path>` or the `OKF_CONFIG` env var):
```yaml
server:
  # Localhost by default. Bind "0.0.0.0" (or your tailnet IP) only when you
  # mean to expose the server — and set a bearer token first (see Security).
  host: "127.0.0.1"
  port: 8080

paths:
  bundles_dir: "./bundles"
  # Must name an existing bundle; validated at startup (fail-fast).
  default_bundle: "shared"

auth:
  # Optional bearer token. Absent = fail-open (no auth, Helix-style).
  # Prefer the OKF_AUTH_TOKEN env var over writing the token here.
  token: null
```

## Security

- **Bind:** default is `127.0.0.1`. Exposing `0.0.0.0` puts the bundle catalog
  on your LAN — fine behind a firewall/tailnet you trust, but do it
  deliberately and set a token first.
- **Auth:** `Authorization: Bearer <token>`, via `auth.token` in the config
  or the `OKF_AUTH_TOKEN` env var (env wins; blank = not configured).
  No token configured → fail-open, everything passes.
  Token configured → a *wrong* token is always rejected (`401`); reads stay
  open without a token, while write endpoints (coming in Phase 2) require it.
- **Input:** `bundle_id` is whitelisted to `[A-Za-z0-9_-]` and the resolved
  path is canonicalized and verified to stay under `bundles_dir` — `..`,
  separators, and symlink escapes are rejected with `400`.
- **Errors:** `500` responses are generic JSON (`{"error":"..."}`) — no
  filesystem paths or OS error text leak to clients. Full details go to the
  server log.
- **Service user:** the systemd unit runs as a dedicated `okf` user, never
  root (see `scripts/install_systemd.sh`).
Bundle Format
Example bundle: bundles/helix/okf.yaml
yaml
bundle:
  id: "helix"
  description: "OKF bundle for Helix agent"
  version: "0.1.0"

tools:
  - id: "fs_read"
    file: "tools/fs_read.yaml"
  - id: "search"
    file: "tools/search.yaml"
Example tool: bundles/helix/tools/fs_read.yaml
yaml
tool:
  id: "fs_read"
  description: "Read a file from disk"
  schema:
    type: "object"
    properties:
      path:
        type: "string"
    required: ["path"]
REST API
Health Check (returns JSON `{"status":"ok","service":"okf-server","version":"..."}`)
Code
GET /health
Get bundle metadata
Code
GET /bundles/{bundle_id}
Get tool definition
Code
GET /bundles/{bundle_id}/tools/{tool_id}
Example
Code
GET /bundles/helix/tools/fs_read
Running the Server
Development
Code
cargo run

Logging
The server uses structured logging via `tracing` + `tracing-subscriber`.

- Default level: `info`
- Configure via `RUST_LOG` env var:
  - `RUST_LOG=debug cargo run`
  - `RUST_LOG=okf=trace,info cargo run`
- Logs include: server startup, config loading, bundle loads (with tool counts), HTTP requests/responses (via `TraceLayer`), errors, and validation issues.
- Key events are logged with structured fields (bundle_id, tool_id, latency, etc.).

Production (systemd example)
Code
[Unit]
Description=OKF Server

[Service]
ExecStart=/usr/local/bin/okf
WorkingDirectory=/opt/okf
Restart=always
User=okf
Group=okf
NoNewPrivileges=true
ProtectSystem=full
PrivateTmp=true

[Install]
WantedBy=multi-user.target
(`scripts/install_systemd.sh` generates this, creates the `okf` user, and
sets ownership — run it instead of hand-writing the unit.)
Using OKF with Grok‑CLI
Grok‑CLI can fetch tool schemas dynamically:

rust
let url = "http://191.168.1.106:8080/bundles/grok-cli/tools/fs_read";
Use this to:

validate tool calls

route tools

load bundle metadata at startup

support multiple bundles (shared + agent‑specific)

Using OKF with Helix
Helix can load:

its own bundle (helix)

shared bundle (shared)

This allows Helix to:

reuse common tools

keep agent‑specific tools isolated

hot‑reload OKF definitions without restarting the agent

Deployment on Proxmox
Recommended layout on your Dell R630:

Code
191.168.1.106 → OKF Server (this project)
192.168.1.104 → Helix (example)
192.168.1.102 → Ollama (example)
192.168.1.50  → Pi-hole / Adblocker (example)
The OKF server should run in its own LXC/VM for stability and clean networking.

Future Extensions
Versioned bundles (/v1/, /v2/)

Bundle diffing

Tool schema validation

Signed bundles

Rust‑native OKF compiler

Agent capability negotiation

License
Apache‑2.0
