<div align='center'>
<img width="768" alt="logo" src="imgs/synapse_logo.png" />

# Synapse
<em>Configuration Language for Memory Systems</em>

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-green.svg)](#)

[Examples](examples) • [Issues](https://github.com/krypticmouse/synapse/issues)

</div>

---

## Prerequisites

Build the CLI:

```bash
cargo build --release -p synapse-cli
```

The binary is at `./target/release/synapse`.

---

## 1. Hello World — `hello.mnm`

The simplest memory system: store notes and retrieve them.

```bash
# Start the runtime
./target/release/synapse apply examples/hello.mnm
```

> **Note:** Data persists in `./data/`. For a fresh start, run `./target/release/synapse destroy --purge` before applying again.

In a separate terminal:

```bash
# Store some notes
./target/release/synapse emit save '{"content": "Remember to buy milk"}'
./target/release/synapse emit save '{"content": "Meeting at 3pm tomorrow"}'
./target/release/synapse emit save '{"content": "Rust is awesome"}'

# Retrieve all notes (newest first)
./target/release/synapse query GetAll
```

---

## 2. Agent Facts — `agent_facts.mnm`

Fact memory with confidence scoring, contradiction resolution, and daily decay.

```bash
./target/release/synapse apply examples/agent_facts.mnm
```

In a separate terminal:

```bash
# Learn some facts
./target/release/synapse emit learn '{"content": "User prefers dark mode", "source": "stated"}'
./target/release/synapse emit learn '{"content": "User is a Python developer", "source": "inferred"}'
./target/release/synapse emit learn '{"content": "User lives in San Francisco", "source": "stated"}'

# Get top facts by confidence
./target/release/synapse query GetFacts '{"limit_n": 10}'

# Filter by source
./target/release/synapse query BySource '{"source": "stated"}'
```

The `update Fact` block defines:
- **on_access**: updates `accessed_at` whenever a fact is queried
- **on_conflict**: keeps the higher-confidence fact when contradictions arise
- **every 24h**: decays confidence by 5% daily, deletes facts below 0.1

---

## 3. Conversation Memory — `conversation.mnm`

Tiered conversation storage with namespaces, message recording, and session archival.

```bash
./target/release/synapse apply examples/conversation.mnm
```

In a separate terminal:

```bash
# Record a conversation
./target/release/synapse emit message '{"role": "user", "content": "What is Rust?", "session_id": "sess_1"}'
./target/release/synapse emit message '{"role": "assistant", "content": "Rust is a systems programming language.", "session_id": "sess_1"}'
./target/release/synapse emit message '{"role": "user", "content": "How about Python?", "session_id": "sess_1"}'
./target/release/synapse emit message '{"role": "assistant", "content": "Python is great for scripting and ML.", "session_id": "sess_1"}'

# A second session
./target/release/synapse emit message '{"role": "user", "content": "Tell me about memory systems", "session_id": "sess_2"}'

# Get messages from a session
./target/release/synapse query RecentMessages '{"session_id": "sess_1"}'

# Search by role
./target/release/synapse query SearchMessages '{"role": "user"}'

# Archive a session
./target/release/synapse emit archive_session '{"session_id": "sess_1", "summary": "Discussed Rust and Python programming languages"}'

# View summaries
./target/release/synapse query GetSummaries
```

---

## 4. User Profile — `user_profile.mnm`

Preference tracking and interaction logging with sentiment scoring.

```bash
./target/release/synapse apply examples/user_profile.mnm --port 9090
```

In a separate terminal:

```bash
# Set user preferences
./target/release/synapse emit set_preference '{"user_id": "u1", "key": "theme", "value": "dark"}'
./target/release/synapse emit set_preference '{"user_id": "u1", "key": "language", "value": "python"}'
./target/release/synapse emit set_preference '{"user_id": "u1", "key": "editor", "value": "vscode"}'

# Log interactions with sentiment
./target/release/synapse emit interaction '{"user_id": "u1", "topic": "machine learning", "sentiment": 0.9}'
./target/release/synapse emit interaction '{"user_id": "u1", "topic": "web development", "sentiment": 0.6}'
./target/release/synapse emit interaction '{"user_id": "u1", "topic": "databases", "sentiment": 0.8}'

# Query preferences
./target/release/synapse query GetPreferences '{"user_id": "u1"}'

# Get top topics by sentiment
./target/release/synapse query TopTopics '{"user_id": "u1"}'
```

---

## 5. Zep — `zep.mnm`

Temporal knowledge graph memory: episodic events, semantic facts with subject-predicate-object triples, and entity summaries. Uses vector search (Qdrant) and graph (Neo4j) with `vector: auto` and `graph: auto`.

**Requires:** Docker (for Qdrant and Neo4j auto-spawn)

```bash
./target/release/synapse apply examples/zep.mnm
```

In a separate terminal:

```bash
# End a conversation to extract episodes and facts (handler expects session_id + messages)
./target/release/synapse emit conversation_end '{"session_id": "s1", "messages": [{"content": "Alice works at Acme Corp"}]}'

# Hybrid retrieval: vector + graph
./target/release/synapse query GetContext '{"input": "Where does Alice work?", "user_id": "u1"}'

# Entity timeline
./target/release/synapse query EntityHistory '{"entity_name": "Alice"}'

# Related entities via graph
./target/release/synapse query RelatedEntities '{"entity": "Alice"}'
```

---

## 6. Letta — `letta.mnm`

Tiered memory with core blocks (persona/human/system), recall (conversation history), and archival (long-term facts). Vector search only; no graph. Self-editing via `@extern` tools.

**Requires:** Docker (for Qdrant auto-spawn)

```bash
./target/release/synapse apply examples/letta.mnm
```

In a separate terminal:

```bash
# Store conversation messages for recall
./target/release/synapse emit message_received '{"role": "user", "content": "I prefer Python for data science", "session_id": "s1"}'
./target/release/synapse emit message_received '{"role": "assistant", "content": "Noted. Python is great for data science.", "session_id": "s1"}'

# Archive a fact
./target/release/synapse emit archive_request '{"content": "User prefers Python for data science", "source": "user_stated"}'

# Get core memory blocks
./target/release/synapse query GetCoreMemory '{}'

# Search recall (recent conversations)
./target/release/synapse query SearchRecall '{"query": "data science", "session_id": null}'

# Search archival memory
./target/release/synapse query SearchArchival '{"query": "Python preferences"}'
```

---

## 7. SuperMemory — `supermemory.mnm`

Universal memory layer: user profiles, memory facts, document chunks, and connector sync (Notion, GDrive, GitHub). Combines conversation extraction, RAG, and scheduled connector refresh.

**Requires:** Docker (for Qdrant and Neo4j auto-spawn)

```bash
./target/release/synapse apply examples/supermemory.mnm
```

In a separate terminal:

```bash
# Store conversation messages (auto-extracts facts + builds graph)
./target/release/synapse emit message '{"user_id": "u1", "content": "I use TypeScript for frontend work", "container": "user"}'

# Explicit save with subject/predicate/object triple
./target/release/synapse emit save_memory '{"user_id": "u1", "content": "Project X uses Next.js", "container": "project", "subject": "Project X", "predicate": "uses", "object": "Next.js"}'

# Main search (memory + RAG)
./target/release/synapse query Search '{"query": "frontend tech", "user_id": "u1", "container": null}'

# Graph-powered search: find memories connected within 2 hops
./target/release/synapse query GraphSearch '{"query": "Next.js", "user_id": "u1"}'

# Hybrid: vector + graph connectivity
./target/release/synapse query DeepSearch '{"query": "frontend frameworks", "user_id": "u1"}'

# Related entities via Cypher
./target/release/synapse query RelatedEntities '{"topic": "Next.js"}'

# Get user profile
./target/release/synapse query GetProfile '{"user_id": "u1"}'
```

---

## Syntax Highlighting (VS Code / Cursor)

A syntax highlighter for `.mnm` files is available in `extensions/synapse-vscode/`.

**Option 1 — Run in development (quickest):**

1. **File → Open Folder** → select `extensions/synapse-vscode`
2. Press **F5** (or **Run → Start Debugging**) to launch a new window with the extension loaded
3. In the new window, open any `.mnm` file to see syntax highlighting

**Option 2 — Install as a local extension:**

```bash
cd extensions/synapse-vscode
npm install -g @vscode/vsce   # if you don't have vsce
vsce package
code --install-extension synapse-mnm-0.1.0.vsix
```

Restart Cursor/VS Code; `.mnm` files will be highlighted in your normal workspace.

**Option 3 — Multi-root workspace:** Add `extensions/synapse-vscode` to your workspace, press **F5**, then open the Synapse project in the new window.

See `extensions/synapse-vscode/README.md` for more details.

---

## Other CLI Commands

```bash
# Initialize a new project (creates synapse.mnm, synapse.toml, .synapse/)
./target/release/synapse init

# Validate a .mnm file without starting the runtime
./target/release/synapse check examples/hello.mnm

# Show execution plan (what would be created)
./target/release/synapse plan examples/hello.mnm

# Check runtime status
./target/release/synapse status

# View logs
./target/release/synapse logs
./target/release/synapse logs --follow --level debug

# Stop and destroy the runtime
./target/release/synapse destroy
./target/release/synapse destroy --purge  # also deletes data/
```

---

## Python SDK

After building with `maturin`:

```bash
cd crates/synapse-python
PYO3_PYTHON=python3.13 maturin develop
```

```python
from synapse import SynapseClient

client = SynapseClient("http://localhost:8080")

# Emit events
client.emit("save", {"content": "Hello from Python!"})

# Run queries
notes = client.query("GetAll")
print(notes)

# Health check
print(client.health())
print(client.ping())
```

---

## Rust SDK

```rust
use synapse_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("http://localhost:8080");

    // Emit an event
    client.emit("save", serde_json::json!({
        "content": "Hello from Rust!"
    })).await?;

    // Run a query
    let notes = client.query("GetAll", serde_json::json!({})).await?;
    println!("{}", serde_json::to_string_pretty(&notes)?);

    Ok(())
}
```
