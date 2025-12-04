//! JavaScript Runtime Backend Abstraction
//!
//! This module provides an abstraction layer for JavaScript runtime backends.
//!
//! # Current Implementation
//!
//! Currently, only the embedded `deno_core` backend is supported. This uses V8
//! directly embedded in the binary, requiring no external dependencies for users.
//!
//! # Future Possibilities
//!
//! The `JsBackend` trait is designed to allow alternative implementations such as:
//! - External Bun process (would require users to install Bun)
//! - External Deno CLI process (would require users to install Deno)
//! - QuickJS for a smaller binary footprint
//!
//! These are not currently implemented to keep the editor self-contained.

use crate::services::plugins::api::{EditorStateSnapshot, PluginCommand, PluginResponse};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// The embedded deno_core backend (default, only supported backend)
pub mod deno_core_backend;

/// Pending response senders type alias for convenience
pub type PendingResponses = Arc<
    std::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<PluginResponse>>>,
>;

/// JavaScript Runtime Backend Trait
///
/// This trait abstracts the JavaScript runtime, allowing different backends
/// to be used interchangeably. Currently only `deno_core` is implemented.
///
/// Note: This trait does NOT require `Send` because JavaScript runtimes
/// (like V8) are typically not thread-safe. The runtime is designed to
/// run on a dedicated plugin thread.
#[allow(async_fn_in_trait)]
pub trait JsBackend {
    /// Create a new backend instance with the given configuration
    fn new(
        state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
        command_sender: std::sync::mpsc::Sender<PluginCommand>,
        pending_responses: PendingResponses,
    ) -> Result<Self>
    where
        Self: Sized;

    /// Load and execute a TypeScript/JavaScript module file
    async fn load_module(&mut self, path: &str, plugin_source: &str) -> Result<()>;

    /// Execute a global function by name (for plugin actions)
    async fn execute_action(&mut self, action_name: &str) -> Result<()>;

    /// Emit an event to all registered handlers
    ///
    /// Returns `Ok(true)` if all handlers returned true, `Ok(false)` if any returned false.
    async fn emit(&mut self, event_name: &str, event_data: &str) -> Result<bool>;

    /// Check if any handlers are registered for an event
    fn has_handlers(&self, event_name: &str) -> bool;

    /// Deliver a response to a pending async operation
    fn deliver_response(&self, response: PluginResponse);

    /// Send a status message to the editor UI
    fn send_status(&mut self, message: String);

    /// Get the pending responses handle
    fn pending_responses(&self) -> &PendingResponses;
}

// Re-export the default backend type
pub use deno_core_backend::DenoCoreBackend;

/// The selected backend type (currently always deno_core)
pub type SelectedBackend = DenoCoreBackend;

/// Get the name of the current JS backend
pub fn backend_name() -> &'static str {
    "deno_core (embedded V8)"
}

/// Check if the selected runtime is available on the system
pub fn check_runtime_available() -> Result<()> {
    // deno_core is always available (embedded)
    Ok(())
}

/// Create a new backend instance
pub fn create_backend(
    state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
    command_sender: std::sync::mpsc::Sender<PluginCommand>,
    pending_responses: PendingResponses,
) -> Result<SelectedBackend> {
    SelectedBackend::new(state_snapshot, command_sender, pending_responses)
}
