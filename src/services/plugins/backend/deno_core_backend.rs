//! deno_core Runtime Backend
//!
//! This module wraps the existing TypeScriptRuntime (using embedded V8 via deno_core)
//! to conform to the JsBackend trait.
//!
//! This is the default backend - it's bundled into the binary and requires no
//! external dependencies.

use crate::services::plugins::api::{EditorStateSnapshot, PluginCommand, PluginResponse};
use crate::services::plugins::backend::{JsBackend, PendingResponses};
use anyhow::Result;
use std::sync::{Arc, RwLock};

// Re-export the actual runtime implementation
pub use crate::services::plugins::runtime::TypeScriptRuntime as DenoCoreBakedRuntime;

/// deno_core backend - wraps the existing TypeScriptRuntime
pub struct DenoCoreBackend {
    runtime: DenoCoreBakedRuntime,
}

impl DenoCoreBackend {
    /// Create a new deno_core backend
    pub fn create(
        state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
        command_sender: std::sync::mpsc::Sender<PluginCommand>,
        pending_responses: PendingResponses,
    ) -> Result<Self> {
        let runtime = DenoCoreBakedRuntime::with_state_and_responses(
            state_snapshot,
            command_sender,
            pending_responses,
        )?;

        Ok(Self { runtime })
    }

    /// Get direct access to the underlying runtime (for advanced use cases)
    pub fn inner(&self) -> &DenoCoreBakedRuntime {
        &self.runtime
    }

    /// Get mutable access to the underlying runtime
    pub fn inner_mut(&mut self) -> &mut DenoCoreBakedRuntime {
        &mut self.runtime
    }
}

impl JsBackend for DenoCoreBackend {
    fn new(
        state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
        command_sender: std::sync::mpsc::Sender<PluginCommand>,
        pending_responses: PendingResponses,
    ) -> Result<Self> {
        Self::create(state_snapshot, command_sender, pending_responses)
    }

    async fn load_module(&mut self, path: &str, plugin_source: &str) -> Result<()> {
        self.runtime.load_module_with_source(path, plugin_source).await
    }

    async fn execute_action(&mut self, action_name: &str) -> Result<()> {
        self.runtime.execute_action(action_name).await
    }

    async fn emit(&mut self, event_name: &str, event_data: &str) -> Result<bool> {
        self.runtime.emit(event_name, event_data).await
    }

    fn has_handlers(&self, event_name: &str) -> bool {
        self.runtime.has_handlers(event_name)
    }

    fn deliver_response(&self, response: PluginResponse) {
        self.runtime.deliver_response(response)
    }

    fn send_status(&mut self, message: String) {
        self.runtime.send_status(message)
    }

    fn pending_responses(&self) -> &PendingResponses {
        self.runtime.pending_responses()
    }
}
