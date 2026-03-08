# Synapse

Configuration Language for Memory Systems

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
