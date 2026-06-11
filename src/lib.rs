//! cqrs-framework: Command Query Responsibility Segregation framework with separate read/write models.

use std::collections::HashMap;
use std::marker::PhantomData;

pub trait Command: Send + Sync {
    type AggregateId;
    fn aggregate_id(&self) -> &Self::AggregateId;
}

pub trait Query: Send + Sync {
    type Result;
}

pub trait CommandHandler<C: Command> {
    fn handle(&self, command: C) -> Result<Vec<String>, String>;
}

pub trait QueryHandler<Q: Query> {
    fn handle(&self, query: Q) -> Q::Result;
}

pub struct CqrsFramework<C, Q> {
    write_model: HashMap<String, Vec<String>>,
    read_model: HashMap<String, String>,
    _cmd: PhantomData<C>,
    _query: PhantomData<Q>,
}

impl<C: Command, Q: Query> CqrsFramework<C, Q> {
    pub fn new() -> Self {
        Self {
            write_model: HashMap::new(),
            read_model: HashMap::new(),
            _cmd: PhantomData,
            _query: PhantomData,
        }
    }

    pub fn dispatch_command(&mut self, aggregate_id: &str, event: String) {
        self.write_model
            .entry(aggregate_id.to_string())
            .or_default()
            .push(event.clone());
        // Project to read model
        self.read_model.insert(aggregate_id.to_string(), event);
    }

    pub fn query_read_model(&self, aggregate_id: &str) -> Option<&str> {
        self.read_model.get(aggregate_id).map(|s| s.as_str())
    }

    pub fn command_count(&self, aggregate_id: &str) -> usize {
        self.write_model.get(aggregate_id).map(|v| v.len()).unwrap_or(0)
    }
}

impl<C: Command, Q: Query> Default for CqrsFramework<C, Q> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CommandBus {
    handlers: HashMap<String, Box<dyn Fn(&str) -> Result<(), String> + Send + Sync>>,
}

impl CommandBus {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, command_type: &str, handler: Box<dyn Fn(&str) -> Result<(), String> + Send + Sync>) {
        self.handlers.insert(command_type.to_string(), handler);
    }

    pub fn dispatch(&self, command_type: &str, payload: &str) -> Result<(), String> {
        match self.handlers.get(command_type) {
            Some(handler) => handler(payload),
            None => Err(format!("No handler for command type: {}", command_type)),
        }
    }
}

impl Default for CommandBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CreateUser {
        id: String,
    }

    impl Command for CreateUser {
        type AggregateId = String;
        fn aggregate_id(&self) -> &Self::AggregateId { &self.id }
    }

    struct GetUser;
    impl Query for GetUser {
        type Result = Option<String>;
    }

    #[test]
    fn test_cqrs_dispatch() {
        let mut framework: CqrsFramework<CreateUser, GetUser> = CqrsFramework::new();
        framework.dispatch_command("user-1", "UserCreated".to_string());
        assert_eq!(framework.query_read_model("user-1"), Some("UserCreated"));
        assert_eq!(framework.command_count("user-1"), 1);
    }

    #[test]
    fn test_command_bus() {
        let mut bus = CommandBus::new();
        bus.register("create_order", Box::new(|payload| {
            if payload.is_empty() {
                Err("empty payload".to_string())
            } else {
                Ok(())
            }
        }));
        assert!(bus.dispatch("create_order", "order:42").is_ok());
        assert!(bus.dispatch("create_order", "").is_err());
        assert!(bus.dispatch("unknown", "x").is_err());
    }
}
