<div align="center">
<img width="768" alt="Synapse logo" src="imgs/synapse_logo.png" />

# Synapse

<em>Configuration language for memory systems</em>

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

[Documentation](#documentation) · [Quick start](#quick-start) · [Examples](examples) · [Issues](https://github.com/krypticmouse/synapse/issues)

</div>

---

Synapse is a domain-specific language (`.mnm` files) and a Rust runtime for building **memory systems** for AI agents. You declare schemas, handlers, queries, update rules, and channels; the runtime talks to relational, vector, and graph backends (SQLite, Qdrant, Neo4j, and others), optional LLM extraction, and an HTTP API.

---

## Documentation

Full documentation (language guide, tutorials, examples walkthroughs, CLI, HTTP API, backends, SDKs) lives in the **`docs/`** Next.js site:

```bash
cd docs
npm install
npm run dev          # http://localhost:3000
npm run build        # static export → docs/out/
```

For production link previews and canonical URLs, set `NEXT_PUBLIC_SITE_URL` when building (see `docs/lib/site.ts`).

The in-repo source is Markdown/MDX under `docs/content/`. Everything that used to live only in this README (DSL reference, architecture notes, API tables) now lives there and stays easier to maintain.

---

## Prerequisites

- **Rust** 1.70+ ([rustup](https://rustup.rs))

Build the CLI:

```bash
cargo build --release -p synapse-cli
```

Binary: `target/release/synapse` (or `synapse.exe` on Windows).

---

## Quick start

```bash
./target/release/synapse apply examples/hello.mnm

# In another terminal
./target/release/synapse emit save '{"content": "Remember to buy milk"}'
./target/release/synapse query GetAll
```

Data is written under **`./data/`** by default. For a clean slate: `./target/release/synapse destroy --purge`.

More tutorials: **docs** → Tutorials, or the [`examples/`](examples) directory.

---

## Examples

| File | What it shows |
|------|----------------|
| [`examples/hello.mnm`](examples/hello.mnm) | Minimal notes store + query |
| [`examples/agent_facts.mnm`](examples/agent_facts.mnm) | Facts, confidence, decay |
| [`examples/conversation.mnm`](examples/conversation.mnm) | Sessions and archival |
| [`examples/user_profile.mnm`](examples/user_profile.mnm) | Preferences and interactions |
| [`examples/zep.mnm`](examples/zep.mnm) | Temporal KG (Docker backends) |
| [`examples/letta.mnm`](examples/letta.mnm) | Tiered memory (Docker) |
| [`examples/supermemory.mnm`](examples/supermemory.mnm) | Multi-container memory layer |
| [`examples/multi_backend.mnm`](examples/multi_backend.mnm) | Named vector/graph backends |
| [`examples/channels.mnm`](examples/channels.mnm) | Channel ingestion |

Exact `emit` / `query` payloads for each file are documented in **Examples** and **Tutorials** in `docs/`. Several samples expect **Docker** when using `auto(...)` for Qdrant or Neo4j.

---

## CLI (cheat sheet)

```text
synapse apply <file.mnm> [--port N]   # Run runtime from a .mnm file
synapse emit <event> '<json>'        # Emit an event
synapse query <name> '<json>'        # Run a named query

synapse inspect [--backend …]        # DB / record overview
synapse clear                        # Clear records (backends)
synapse init | check | plan | status | reload | logs | destroy [--purge]
```

Full command reference: **`docs`** → Reference → CLI.

---

## HTTP API

The runtime serves REST endpoints (default **http://localhost:8080**), including `/health`, `/emit`, `/query`, `/inspect`, `/reload`, and `/clear`. Request/response shapes are documented under **Reference → HTTP API** in `docs/`.

---

## Repository layout

```text
synapse/
├── crates/
│   ├── synapse-dsl/       # Parser, AST, types
│   ├── synapse-runtime/   # Interpreter, storage, HTTP, LLM, Docker helpers
│   ├── synapse-channels/  # Ingestion connectors
│   ├── synapse-cli/       # CLI binary
│   └── synapse-client/    # Rust HTTP client
├── crates/synapse-python/ # Python bindings (see crate README; workspace exclude)
├── docs/                  # Documentation site (Next.js + MDX)
├── examples/              # Example .mnm programs
├── extensions/synapse-vscode/
└── imgs/
```

**Flow:** `.mnm` → parse & type-check (`synapse-dsl`) → runtime (`synapse-runtime`) → SQLite / vector / graph backends. LLM calls use **rig-core** (OpenAI-compatible env vars). See **Language → Overview** and **Reference → Storage backends** in `docs/` for diagrams and detail.

---

## SDKs

**Rust** — `synapse_client::Client` in `crates/synapse-client` (see docs → Reference → SDKs).

**Python** — `crates/synapse-python` (PyO3 / maturin). From that directory:

```bash
PYO3_PYTHON=python3.13 maturin develop
```

```python
from synapse import SynapseClient
client = SynapseClient("http://localhost:8080")
client.emit("save", {"content": "Hello from Python!"})
print(client.query("GetAll"))
```

---

## Editor support

**[Synapse (.mnm) Syntax](https://open-vsx.org/extension/krypticmouse/synapse-mnm)** on Open VSX — works in VS Code and Cursor. Local packaging: `extensions/synapse-vscode/README.md`.

---

## License

Apache 2.0 — see [LICENSE](LICENSE).
