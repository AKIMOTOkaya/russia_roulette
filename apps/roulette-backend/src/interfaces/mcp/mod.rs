//! Model Context Protocol (MCP) interface layer components.

#![forbid(unsafe_code)]

pub mod dispatcher;
pub mod schema;
pub mod server;

pub use dispatcher::{McpDispatcher, McpError};
pub use schema::McpToolDefinition;
pub use server::RussianRouletteMcpServer;
