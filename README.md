<div align='center'>
<img width="768" alt="logo" src="imgs/synapse_logo.png" />

# Synapse
<em>Configuration Language for Memory Systems</em>

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-green.svg)](#)

[Examples](examples) • [DSL Reference](#dsl-reference) • [Architecture](#architecture) • [Issues](https://github.com/krypticmouse/synapse/issues)

</div>

---

Synapse is a domain-specific language (`.mnm` files) and runtime for building memory systems for AI agents. Define schemas, event handlers, queries, and update policies declaratively — Synapse compiles them into a multi-backend runtime backed by SQLite, Qdrant (vector), and Neo4j (graph).

---

 ## Prerequisites

Build the CLI:

```bash
cargo build --release -p synapse-cli
```

The binary is at `./target/release/synapse`.

---

## Quick Start

```bash
# Start a memory system
./target/release/synapse apply examples/hello.mnm

# In another terminal — store and query
./target/release/synapse emit save '{"content": "Remember to buy milk"}'
./target/release/synapse query GetAll
```

> Data persists in `./data/`. For a fresh start: `./target/release/synapse destroy --purge`

---

## Examples

### 1. Hello World — `hello.mnm`

Store notes and retrieve them.

```bash
./target/release/synapse apply examples/hello.mnm

./target/release/synapse emit save '{"content": "Meeting at 3pm tomorrow"}'
./target/release/synapse query GetAll
```

### 2. Agent Facts — `agent_facts.mnm`

Fact memory with confidence scoring, contradiction resolution, and daily decay.

```bash
./target/release/synapse apply examples/agent_facts.mnm

./target/release/synapse emit learn '{"content": "User prefers dark mode", "source": "stated"}'
./target/release/synapse query GetFacts '{"limit_n": 10}'
./target/release/synapse query BySource '{"source": "stated"}'
```

### 3. Conversation Memory — `conversation.mnm`

Tiered conversation storage with session archival.

```bash
./target/release/synapse apply examples/conversation.mnm

./target/release/synapse emit message '{"role": "user", "content": "What is Rust?", "session_id": "sess_1"}'
./target/release/synapse query RecentMessages '{"session_id": "sess_1"}'
./target/release/synapse emit archive_session '{"session_id": "sess_1", "summary": "Discussed Rust"}'
```

### 4. User Profile — `user_profile.mnm`

Preference tracking and interaction logging with sentiment scoring.

```bash
./target/release/synapse apply examples/user_profile.mnm --port 9090

./target/release/synapse emit set_preference '{"user_id": "u1", "key": "theme", "value": "dark"}'
./target/release/synapse emit interaction '{"user_id": "u1", "topic": "machine learning", "sentiment": 0.9}'
./target/release/synapse query TopTopics '{"user_id": "u1"}'
```

### 5. Zep — `zep.mnm`

Temporal knowledge graph: episodic events, semantic facts with SPO triples, entity summaries. Uses vector search (Qdrant) and graph (Neo4j).

**Requires:** Docker

```bash
./target/release/synapse apply examples/zep.mnm

./target/release/synapse emit conversation_end '{
  "session_id": "s1",
  "messages": [
    {"role": "user", "content": "Where does Alice work?"},
    {"role": "assistant", "content": "Alice works at Acme Corp as a senior engineer."},
    {"role": "user", "content": "Who is her manager?"},
    {"role": "assistant", "content": "Bob is her manager at Acme Corp."}
  ]
}'

./target/release/synapse query GetContext '{"input": "Who is the manager of Alice?"}'
./target/release/synapse query EntityHistory '{"entity_name": "Alice"}'
./target/release/synapse query RelatedEntities '{"entity": "Alice"}'
./target/release/synapse inspect
```

### 6. Letta — `letta.mnm`

Tiered memory: core blocks, recall, and archival. Vector search only.

**Requires:** Docker

```bash
./target/release/synapse apply examples/letta.mnm

./target/release/synapse emit message_received '{"role": "user", "content": "I prefer Python for data science", "session_id": "s1"}'
./target/release/synapse query SearchArchival '{"query": "Python preferences"}'
```

### 7. SuperMemory — `supermemory.mnm`

Universal memory layer: user profiles, facts, document chunks, connector sync.

**Requires:** Docker

```bash
./target/release/synapse apply examples/supermemory.mnm

./target/release/synapse emit message '{"user_id": "u1", "content": "I use TypeScript for frontend work", "container": "user"}'
./target/release/synapse query DeepSearch '{"query": "frontend frameworks", "user_id": "u1"}'
```

---

## CLI Commands

```bash
synapse apply <file.mnm>           # Start runtime from a .mnm file
synapse apply <file.mnm> --port N  # Start on a custom port

synapse emit <event> '<json>'      # Emit an event
synapse query <name> '<json>'      # Run a named query

synapse inspect                    # Show all databases and record counts
synapse inspect --backend sqlite   # Inspect a specific backend
synapse clear                      # Delete all records from all backends

synapse init                       # Scaffold a new project
synapse check <file.mnm>           # Validate without starting
synapse plan <file.mnm>            # Show execution plan
synapse status                     # Check if runtime is running
synapse reload                     # Hot-reload DSL from source

synapse logs                       # View logs
synapse logs --follow --level debug

synapse destroy                    # Stop runtime
synapse destroy --purge            # Stop and delete data/
```

---

## HTTP API

The runtime exposes a REST API (default `localhost:8080`):

| Method | Path | Body | Description |
|--------|------|------|-------------|
| `GET` | `/health` | — | Returns `{ status, uptime_secs }` |
| `GET` | `/status` | — | Lists handlers, queries, memories, uptime |
| `GET` | `/inspect` | — | Dumps all backend contents |
| `POST` | `/emit` | `{ "event": "...", "payload": {...} }` | Triggers an event handler |
| `POST` | `/query` | `{ "query": "...", "params": {...} }` | Executes a named query |
| `POST` | `/reload` | — | Hot-reloads the DSL source file |
| `POST` | `/clear` | — | Clears all records from all backends |

---

## DSL Reference

Synapse programs are written in `.mnm` files. A program consists of an optional `config` block followed by top-level declarations inside an optional `namespace`.

### Config

```
config {
  storage: sqlite("./data/myapp.db")
  vector: auto                             # auto-spawns Qdrant via Docker
  graph: neo4j("bolt://localhost:7687")    # or: auto, none
  extractor: openai("gpt-4o")
  embedding: openai("text-embedding-3-small")
}
```

| Key | Values | Description |
|-----|--------|-------------|
| `storage` | `sqlite("path")`, `none` | Relational backend |
| `vector` | `qdrant("url")`, `auto`, `embedded`, `none` | Vector backend |
| `graph` | `neo4j("url")`, `auto`, `none` | Graph backend |
| `extractor` | `openai("model")` | LLM for `extract()` and `summarize()` |
| `embedding` | `openai("model")` | Embedding model for semantic search |

`auto` will auto-spawn the service via Docker. Host/port default to `localhost:8080` and can be overridden with `--port` on the CLI.

### Namespace

Groups related declarations together:

```
namespace myapp {
  memory ... { }
  on ... { }
  query ... { }
}
```

### Types

| Type | Syntax | Example |
|------|--------|---------|
| String | `string` | `name: string` |
| Integer | `int` | `count: int` |
| Float | `float` | `score: float` |
| Bounded float | `float[min,max]` | `confidence: float[0,1]` |
| Boolean | `bool` | `active: bool` |
| Timestamp | `timestamp` | `created_at: timestamp` |
| Optional | `T?` | `deleted_at: timestamp?` |
| Array | `T[]` | `tags: string[]` |
| Named type | `TypeName` | `author: User` |

### Memory

Defines a schema for stored records. Each memory type gets a table in the relational backend, a collection in the vector backend, and nodes in the graph backend.

```
memory Fact {
  content: string
  subject: string
  predicate: string
  object: string
  confidence: float[0,1]
  valid_from: timestamp
  valid_until: timestamp?
  superseded_by: string?
}
```

**Decorators:**

Decorators can be placed inline on a field or standalone inside the memory block.

```
memory Note {
  content: string
  confidence: float[0,1]
  tags: string[] = []            # default value
  @extern source: string         # externally-managed field

  @index content                 # creates a SQLite index on the column
  @invariant confidence > 0      # enforced at store() time
}
```

`@index <field>` creates a `CREATE INDEX IF NOT EXISTS` on the corresponding SQLite column. This speeds up `WHERE` filters and is also used as the conflict key for `on_conflict` rules.

`@invariant <expr>` declares a constraint that is evaluated against every record before `store()` writes it. If the expression evaluates to false, the store is rejected with an error. The expression can reference any field of the record.

Every record automatically gets `_id` (UUID) and `_type` (memory name) virtual fields.

### Handlers (`on`)

Event handlers run when an event is emitted. Parameters are bound from the JSON payload.

```
on conversation_end(session_id: string, messages: Message[]) {
  let episode = Episode {
    session_id: session_id,
    content: messages |> summarize(),
    created_at: now()
  }
  store(episode)

  let facts = messages |> extract()
  for fact in facts {
    store(fact)
  }
}
```

### Queries

Named queries with typed parameters. The body specifies `from`, optional `where`, `order by`, and `limit`.

```
query GetContext(input: string): Fact[] {
  from Fact
  where semantic_match(input, threshold: 0.6)
    and graph_match(input, hops: 2)
    and valid_until == null
  order by _score desc
  limit 10
}

query EntityHistory(entity_name: string): Fact[] {
  from Fact
  where subject == entity_name or object == entity_name
  order by valid_from asc
}
```

Queries are callable from handlers and update rules as regular functions:

```
update Entity {
  every 6h {
    let facts = EntityHistory(name)   # calls the named query
    summary = facts |> summarize()
  }
}
```

**Where-clause functions:**

| Function | Purpose |
|----------|---------|
| `semantic_match(input, threshold: 0.6)` | Vector similarity search via Qdrant |
| `graph_match(input, hops: 2)` | Graph traversal via Neo4j |
| `cypher("MATCH ... RETURN ...")` | Raw Cypher query; `$params` resolved from scope |
| `regex(field, "pattern")` | In-memory regex filter |
| `sql("SELECT ...")` | Raw SQL on the relational backend |

Standard comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`) are pushed to the backend. `and` is conjunction, `or` is disjunction.

**Virtual field `_score`:** When using `semantic_match`, each result gets a `_score` (0.0–1.0) from vector similarity. Use `order by _score desc` to rank by relevance. This field is computed in-memory and not stored.

### Update Rules

Define lifecycle behavior for a memory type.

```
update Fact {
  on_access {
    accessed_at = now()
  }

  on_conflict(old, new) {
    if new.confidence > old.confidence {
      old.valid_until = new.valid_from
      old.superseded_by = new.id
      store(new)
    } else {
      discard(new)
    }
  }

  every 24h {
    confidence = confidence * 0.95
    if confidence < 0.1 {
      delete()
    }
  }
}
```

| Rule | Trigger |
|------|---------|
| `on_access { ... }` | When a record is read |
| `on_conflict(old, new) { ... }` | When a conflicting record is stored |
| `every <duration> { ... }` | Periodically (e.g. `6h`, `24h`, `1w`) |

Duration units: `s` (seconds), `m` (minutes), `h` (hours), `d` (days), `w` (weeks).

### Policies

Standalone rules not tied to a specific memory (same rule types as `update`):

```
policy Cleanup {
  every 1d {
    # runs daily
  }
}
```

### Extern Functions

Declare external function signatures. At runtime, calls to extern functions are routed to the LLM, which simulates the function:

```
@extern fn search(query: string, limit: int): ArchivalEntry[]
@extern fn core_memory_append(section: string, content: string)
```

### Expressions

**Literals:** `42`, `3.14`, `"hello"`, `'world'`, `true`, `false`, `null`, `24h`, `1w`

**Operators:** `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `and`, `or`, `not`, `-` (unary)

**Access:** `obj.field`, `obj?.field` (optional chain), `arr[0]`

**Struct construction:**

```
Note { content: "hello", created_at: now() }
```

**Pipe operator** passes the left value as the first argument to the right:

```
messages |> extract() |> store()
# equivalent to: store(extract(messages))

facts |> filter(f => f.confidence > 0.7) |> map(f => f.content)
```

**Lambdas:**

```
x => x.content
(a, b) => a + b
```

**Inline queries** (as expressions):

```
let recent = from Message where session_id == sid order by created_at desc limit 5
```

### Statements

```
let x = expr              # variable binding
x = expr                  # assignment
obj.field = expr           # field assignment
if cond { ... } else { }  # conditional
for item in list { ... }   # iteration
return expr                # early return
expr                       # bare expression (e.g. function call)
```

### Built-in Functions

| Function | Description |
|----------|-------------|
| `now()` | Current UTC timestamp |
| `store(record)` | Validate `@invariant` constraints, then store to all configured backends |
| `delete(record?)` | Delete a record (or current record in update rules) |
| `archive()` | Mark current record as `_archived: true` |
| `discard(record?)` | No-op; explicitly ignore a record |
| `supersede(old, new)` | Mark `old` as superseded by `new`, store both |
| `len(arr\|str)` | Length of array or string |
| `min(a, b)` | Minimum of two numbers |
| `max(a, b)` | Maximum of two numbers |
| `extract(text)` | LLM fact extraction; returns `Fact[]` with SPO triples |
| `summarize(text)` | LLM summarization; returns string |
| `semantic_match(a, b, threshold?)` | Embedding cosine similarity; returns bool |
| `regex(text, pattern)` | Regex match; returns bool |
| `sql(query)` | Raw SQL query; returns records |
| `graph_match(input, hops?)` | Neo4j graph traversal; returns IDs |
| `cypher(query, ...)` | Raw Cypher query; returns IDs |
| `emit(event, ...)` | Emit an event (triggers a handler) |

**Pipe-only functions** (used as `array |> fn()`):

| Function | Description |
|----------|-------------|
| `map(fn)` | Transform each element |
| `filter(fn)` | Keep elements matching predicate |
| `each(fn)` | Iterate (side effects); returns null |
| `group_by(field?)` | Group by field name; returns array of arrays |
| `store_as(TypeName)` | Store all records with a given type name |
| `delete_originals` | Delete all records in the array |

---

## Architecture

### Crate Layout

```
synapse/
├── crates/
│   ├── synapse-core/       # Lexer, parser, AST, type system, type checker
│   ├── synapse-runtime/    # Interpreter, storage backends, HTTP server, LLM
│   ├── synapse-cli/        # CLI binary (apply, emit, query, inspect, ...)
│   ├── synapse-sdk/        # Rust HTTP client library
│   └── synapse-python/     # Python bindings via PyO3
└── examples/               # .mnm example files
```

### Pipeline: From `.mnm` to Running Server

```
  .mnm file
     │
     ▼
  ┌──────────┐     ┌───────────┐     ┌──────────────┐
  │  Parser   │────▶│ Type Check │────▶│   Runtime    │
  │(synapse-  │     │(synapse-  │     │(synapse-     │
  │  core)    │     │  core)    │     │  runtime)    │
  └──────────┘     └───────────┘     └──────────────┘
                                           │
                              ┌────────────┼────────────┐
                              ▼            ▼            ▼
                         ┌────────┐  ┌─────────┐  ┌────────┐
                         │ SQLite │  │ Qdrant  │  │ Neo4j  │
                         │(relat.)│  │(vector) │  │(graph) │
                         └────────┘  └─────────┘  └────────┘
```

1. **Parse** — `synapse-core` lexes and parses `.mnm` into an AST (`Program`)
2. **Type check** — validates schemas, handler params, query bodies
3. **Runtime init** — `Runtime` registers handlers, queries, update rules; `StorageManager` connects backends; `PolicyScheduler` starts periodic rules
4. **HTTP server** — Axum-based server exposes `/emit`, `/query`, `/inspect`, etc.

### Storage Backends

**SQLite** (relational) — primary record store. Stores all fields as columns. Handles standard `WHERE` conditions, `ORDER BY`, `LIMIT`. Tables and indexes are created automatically from memory schemas — each `@index` declaration produces a `CREATE INDEX IF NOT EXISTS` on the corresponding column.

**Qdrant** (vector) — vector similarity search. On `store()`, the record's `content` field is embedded and stored as a vector point. On query, `semantic_match` embeds the input and finds nearest neighbors, returning `(id, score)` pairs.

**Neo4j** (graph) — knowledge graph. On `store()`, if a record has `subject`/`predicate`/`object` fields, a triple `(subject)-[predicate]->(object)` is created and linked to the record via `HAS_FACT` edges. On query, `graph_match` and `cypher` traverse the graph and return matching IDs.

### Multi-Backend Query Pipeline

When a query uses `semantic_match`, `graph_match`, or `cypher`, the runtime orchestrates across backends:

```
 ┌─────────────┐   ┌─────────────┐
 │ graph_match │   │semantic_match│
 │  (Neo4j)    │   │  (Qdrant)   │
 └──────┬──────┘   └──────┬──────┘
        │  IDs             │  IDs + scores
        └────────┬─────────┘
                 ▼
           Union of IDs
                 │
                 ▼
    ┌────────────────────────┐
    │  SQLite (conditions,   │
    │  no limit applied yet) │
    └────────────┬───────────┘
                 │
                 ▼
    Filter by candidate IDs
                 │
                 ▼
    Attach _score from Qdrant
                 │
                 ▼
    Sort by _score (in memory)
                 │
                 ▼
    Apply limit ──▶ Results
```

Key behaviors:
- Graph and semantic candidate IDs are **unioned** (either backend can contribute)
- `limit` is **deferred** until after scoring and sorting when semantic/graph backends are active
- If SQLite has no results, records are fetched directly from graph/vector backends as a fallback
- `_score` is a virtual field computed in-memory, never persisted

### LLM Integration

Synapse uses [rig-core](https://github.com/0xPlaygrounds/rig) for LLM calls (OpenAI API via environment credentials).

**Extractor** (`extract(text)`) — sends text to the configured LLM model with a structured extraction prompt. Returns an array of `Fact` records, each with `content`, `subject`, `predicate`, `object`, `confidence`, and `valid_from`.

**Summarizer** (`summarize(text)`) — sends text with a summarization prompt. Returns a string.

**Embedder** (`EmbeddingClient`) — generates embeddings for semantic search and `semantic_match()`.

**Extern functions** — when calling an `@extern fn`, the runtime builds a prompt describing the function signature and arguments, sends it to the LLM, and parses the JSON response.

### Docker Auto-Spawn

When `vector: auto` or `graph: auto` is set in the config:

- **Qdrant** — spawns `synapse-qdrant` container (image: `qdrant/qdrant`), ports 6333 (HTTP) + 6334 (gRPC), data persisted in `./data/qdrant_storage`
- **Neo4j** — spawns `synapse-neo4j` container (image: `neo4j:latest`), ports 7687 (Bolt) + 7474 (HTTP), auth disabled, data persisted in `./data/neo4j_data`

If the container exists but is stopped, it is restarted. The runtime waits for the service to be reachable before proceeding.

---

## SDKs

### Python

```bash
cd crates/synapse-python
PYO3_PYTHON=python3.13 maturin develop
```

```python
from synapse import SynapseClient

client = SynapseClient("http://localhost:8080")
client.emit("save", {"content": "Hello from Python!"})
notes = client.query("GetAll")
print(notes)
```

### Rust

```rust
use synapse_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("http://localhost:8080");

    client.emit("save", serde_json::json!({
        "content": "Hello from Rust!"
    })).await?;

    let notes = client.query("GetAll", serde_json::json!({})).await?;
    println!("{}", serde_json::to_string_pretty(&notes)?);

    Ok(())
}
```

---

## Syntax Highlighting

The **[Synapse (.mnm) Syntax](https://open-vsx.org/extension/krypticmouse/synapse-mnm)** extension is published on OpenVSX and works in both **Cursor** and **VS Code**.

**Install from the marketplace:**

1. Open the Extensions panel (`Cmd+Shift+X` / `Ctrl+Shift+X`)
2. Search for **"Synapse (.mnm) Syntax"**
3. Click **Install**

All `.mnm` files will be highlighted automatically.

**Or install from source:**

```bash
cd extensions/synapse-vscode
npm install -g @vscode/vsce
vsce package
code --install-extension synapse-mnm-0.1.1.vsix
```
