//! Model Context Protocol (MCP) interface layer components.

#![forbid(unsafe_code)]

pub mod dispatcher;
pub mod schema;

pub use dispatcher::{McpDispatcher, McpError};
pub use schema::McpToolDefinition;
