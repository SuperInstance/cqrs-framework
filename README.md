# CQRS Framework

**A Command Query Responsibility Segregation (CQRS) framework in Rust** that separates write operations (commands) from read operations (queries), enabling independent optimization of each path — the core pattern behind event-sourced systems.

## Why It Matters

In a traditional CRUD architecture, the same data model handles both writes and reads. This works for simple applications but breaks down at scale: write-heavy workloads need normalized schemas with strict consistency, while read-heavy workloads benefit from denormalized projections optimized for query speed. CQRS solves this by splitting the system into two models:

- **Write model** — Accepts commands, applies business logic, and produces events. This is the source of truth.
- **Read model** — Pre-computed projections of the write model, optimized for specific queries.

This separation is used in production by systems like Amazon, LinkedIn, and Event StoreDB. The trade-off is eventual consistency: the read model lags behind the write model by the projection latency. In return, you gain independent scaling (read replicas can outnumber write nodes), schema flexibility (different read projections for different consumers), and auditability (the event log is an append-only history).

This framework also implements a **Command Bus** — a registry that routes command types to their handlers, decoupling command originators from command processors.

## How It Works

The framework is built around four traits:

**`Command`** — Represents an intent to change state. Each command carries an `AggregateId` that identifies the entity it targets. Commands are immutable value objects; they describe *what should happen*, not *how*.

**`Query`** — Represents a request for information. Queries return a typed `Result` and never modify state. This is enforced at the type level — `QueryHandler::handle` takes `&self`, not `&mut self`.

**`CommandHandler<C>`** — Processes a command and returns either a list of event strings (on success) or an error (on failure). The events are what gets persisted and projected.

**`QueryHandler<Q>`** — Executes a query against the read model and returns the result.

The `CqrsFramework` struct maintains both models internally. When `dispatch_command` is called, it appends the event to the write model's event log (a `Vec<String>` per aggregate) and projects the latest event into the read model. This dual-write pattern is the essence of CQRS: the write model is the authoritative event stream, and the read model is a continuously updated materialized view.

The `CommandBus` provides type-based routing: handlers are registered against string keys, and `dispatch` looks up the appropriate handler. This enables a plugin architecture where new command types can be registered without modifying existing code.

## Quick Start

```rust
use cqrs_framework::{CqrsFramework, Command, Query, CommandBus};

// Define your commands
struct CreateUser { id: String }
impl Command for CreateUser {
    type AggregateId = String;
    fn aggregate_id(&self) -> &Self::AggregateId { &self.id }
}

// Define your queries
struct GetUser;
impl Query for GetUser {
    type Result = Option<String>;
}

// Use the framework
let mut framework: CqrsFramework<CreateUser, GetUser> = CqrsFramework::new();
framework.dispatch_command("user-1", "UserCreated".to_string());
framework.dispatch_command("user-1", "EmailVerified".to_string());

// Read from the projected read model
assert_eq!(framework.query_read_model("user-1"), Some("EmailVerified"));
assert_eq!(framework.command_count("user-1"), 2); // two events in the log

// Use the Command Bus for handler routing
let mut bus = CommandBus::new();
bus.register("create_order", Box::new(|payload| {
    if payload.is_empty() {
        Err("empty payload".to_string())
    } else {
        Ok(())
    }
}));
bus.dispatch("create_order", "order:42").unwrap();
```

## API

### Core Traits
- `Command` — Trait for write operations; requires `aggregate_id()`
- `Query` — Trait for read operations; defines `type Result`
- `CommandHandler<C: Command>` — Handles commands, returns `Result<Vec<String>, String>`
- `QueryHandler<Q: Query>` — Handles queries, returns `Q::Result`

### `CqrsFramework<C, Q>`
- `new() -> Self` — Initialize with empty write and read models
- `dispatch_command(&mut self, aggregate_id: &str, event: String)` — Execute command, append event, project to read model
- `query_read_model(&self, aggregate_id: &str) -> Option<&str>` — Read current state from projection
- `command_count(&self, aggregate_id: &str) -> usize` — Number of events for an aggregate

### `CommandBus`
- `new() -> Self` — Create empty bus
- `register(&mut self, command_type: &str, handler)` — Register a handler for a command type
- `dispatch(&self, command_type: &str, payload: &str) -> Result<(), String>` — Route to handler

## Architecture Notes

This framework provides the CQRS infrastructure for SuperInstance services that require event sourcing — including user management, order processing, and audit logging. The write model's event log integrates with the broader event-sourcing infrastructure for persistence and replay.

See the full architecture: [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md)

## License

MIT
