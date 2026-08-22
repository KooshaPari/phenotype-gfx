//! Runtime plugin loading system.
//!
//! Provides a [`Plugin`] trait, a [`PluginContext`] that is passed to plugins
//! during initialisation, and a [`PluginManager`] that keeps track of loaded
//! plugins and dispatches lifecycle calls.

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during plugin operations.
#[derive(Debug, Clone)]
pub enum PluginError {
    /// A plugin with the given name is already registered.
    AlreadyRegistered(String),
    /// No plugin with the given name is registered.
    NotFound(String),
    /// The directory path could not be read.
    IoError(String),
    /// Plugin initialisation returned an error.
    InitFailed(String),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::AlreadyRegistered(n) => write!(f, "plugin already registered: {n}"),
            PluginError::NotFound(n) => write!(f, "plugin not found: {n}"),
            PluginError::IoError(e) => write!(f, "I/O error: {e}"),
            PluginError::InitFailed(e) => write!(f, "plugin init failed: {e}"),
        }
    }
}

impl std::error::Error for PluginError {}

// ---------------------------------------------------------------------------
// PluginInfo
// ---------------------------------------------------------------------------

/// Descriptive metadata about a plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Human-readable plugin name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Author / maintainer.
    pub author: String,
    /// Free-text description.
    pub description: String,
}

// ---------------------------------------------------------------------------
// Plugin trait
// ---------------------------------------------------------------------------

/// Trait that every runtime plugin must implement.
pub trait Plugin: Send {
    /// Return the unique name of this plugin.
    fn name(&self) -> &str;

    /// Return the version of this plugin.
    fn version(&self) -> &str;

    /// Initialise the plugin with the given [`PluginContext`].
    fn init(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError>;
}

// ---------------------------------------------------------------------------
// PluginContext
// ---------------------------------------------------------------------------

/// Context object handed to plugins during [`Plugin::init`].
pub struct PluginContext {
    /// Opaque renderer handle (reserved for future use).
    pub renderer: (),
    /// Configuration key-value pairs.
    pub config: HashMap<String, String>,
}

impl PluginContext {
    /// Create a new context with an empty renderer and no config.
    pub fn new() -> Self {
        Self {
            renderer: (),
            config: HashMap::new(),
        }
    }
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// PluginManager
// ---------------------------------------------------------------------------

/// Manages the lifecycle of plugins: registration, loading from a directory,
/// and execution.
pub struct PluginManager {
    /// Registered plugin instances keyed by name.
    plugins: Vec<Box<dyn Plugin>>,
    /// Metadata for every registered plugin.
    infos: Vec<PluginInfo>,
}

impl PluginManager {
    /// Create an empty manager.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            infos: Vec::new(),
        }
    }

    /// Register a plugin.  Returns an error if a plugin with the same name is
    /// already registered.
    pub fn register<P: Plugin + 'static>(
        &mut self,
        mut plugin: P,
        info: PluginInfo,
    ) -> Result<(), PluginError> {
        let name = plugin.name().to_owned();
        if self.plugins.iter().any(|p| p.name() == name) {
            return Err(PluginError::AlreadyRegistered(name));
        }
        plugin.init(&mut PluginContext::new())?;
        self.plugins.push(Box::new(plugin));
        self.infos.push(info);
        Ok(())
    }

    /// Load plugins from a directory path.  Currently this is a stub that
    /// returns a list of `.plugin` filenames found in the directory (actual
    /// dynamic loading via `dlopen`/`libloading` is left as a future
    /// extension).
    pub fn load_from_dir(&self, path: &str) -> Result<Vec<String>, PluginError> {
        use std::fs;

        let entries = fs::read_dir(path).map_err(|e| {
            PluginError::IoError(format!("failed to read directory '{path}': {e}"))
        })?;

        let mut files = Vec::new();
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if file_name.ends_with(".plugin") {
                files.push(file_name);
            }
        }
        Ok(files)
    }

    /// Initialise and execute every registered plugin with the given context.
    ///
    /// Returns a list of `(name, Ok(()))` or `(name, Err(message))` pairs.
    pub fn execute_all(
        &mut self,
        ctx: &mut PluginContext,
    ) -> Vec<(String, Result<(), String>)> {
        let mut results = Vec::with_capacity(self.plugins.len());
        for plugin in self.plugins.iter_mut() {
            let name = plugin.name().to_owned();
            let result = plugin.init(ctx).map_err(|e| e.to_string());
            results.push((name, result));
        }
        results
    }

    /// Return metadata for all registered plugins.
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.infos.clone()
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether no plugins are registered.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- helpers ----------------------------------------------------------

    /// A trivial plugin that always initialises successfully.
    struct DummyPlugin {
        name: &'static str,
        version: &'static str,
    }

    impl Plugin for DummyPlugin {
        fn name(&self) -> &str {
            self.name
        }
        fn version(&self) -> &str {
            self.version
        }
        fn init(&mut self, _ctx: &mut PluginContext) -> Result<(), PluginError> {
            Ok(())
        }
    }

    fn dummy_info(name: &str) -> PluginInfo {
        PluginInfo {
            name: name.to_owned(),
            version: "0.1.0".to_owned(),
            author: "Test".to_owned(),
            description: "A test plugin".to_owned(),
        }
    }

    /// A plugin that fails during initialisation.
    struct FailingPlugin;

    impl Plugin for FailingPlugin {
        fn name(&self) -> &str {
            "failing"
        }
        fn version(&self) -> &str {
            "0.0.0"
        }
        fn init(&mut self, _ctx: &mut PluginContext) -> Result<(), PluginError> {
            Err(PluginError::InitFailed("boom".into()))
        }
    }

    // ---- tests ------------------------------------------------------------

    #[test]
    fn register_and_list_plugins() {
        let mut mgr = PluginManager::new();
        mgr.register(
            DummyPlugin { name: "a", version: "1.0" },
            dummy_info("a"),
        )
        .unwrap();
        mgr.register(
            DummyPlugin { name: "b", version: "2.0" },
            dummy_info("b"),
        )
        .unwrap();

        let list = mgr.list_plugins();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "a");
        assert_eq!(list[1].name, "b");
    }

    #[test]
    fn duplicate_register_returns_error() {
        let mut mgr = PluginManager::new();
        mgr.register(
            DummyPlugin { name: "dup", version: "1" },
            dummy_info("dup"),
        )
        .unwrap();

        let err = mgr
            .register(
                DummyPlugin { name: "dup", version: "2" },
                dummy_info("dup"),
            )
            .unwrap_err();

        assert!(
            matches!(err, PluginError::AlreadyRegistered(ref n) if n == "dup"),
            "expected AlreadyRegistered, got: {err}"
        );
    }

    #[test]
    fn execute_all_runs_all_plugins() {
        let mut mgr = PluginManager::new();
        mgr.register(DummyPlugin { name: "x", version: "1" }, dummy_info("x"))
            .unwrap();
        mgr.register(DummyPlugin { name: "y", version: "1" }, dummy_info("y"))
            .unwrap();

        let results = mgr.execute_all(&mut PluginContext::new());
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(_, r)| r.is_ok()));
    }

    #[test]
    fn execute_all_propagates_init_failure() {
        let mut mgr = PluginManager::new();
        mgr.register(
            DummyPlugin { name: "ok", version: "1" },
            dummy_info("ok"),
        )
        .unwrap();
        mgr.register(FailingPlugin, dummy_info("failing")).unwrap();

        let results = mgr.execute_all(&mut PluginContext::new());
        assert_eq!(results.len(), 2);

        let ok_result = results.iter().find(|(n, _)| n == "ok").unwrap();
        assert!(ok_result.1.is_ok());

        let fail_result = results.iter().find(|(n, _)| n == "failing").unwrap();
        assert!(fail_result.1.is_err());
        assert!(fail_result.1.as_ref().unwrap_err().contains("boom"));
    }

    #[test]
    fn len_and_is_empty_reflect_registrations() {
        let mut mgr = PluginManager::new();
        assert!(mgr.is_empty());
        assert_eq!(mgr.len(), 0);

        mgr.register(DummyPlugin { name: "one", version: "1" }, dummy_info("one"))
            .unwrap();
        assert!(!mgr.is_empty());
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn plugin_context_default_is_empty() {
        let ctx = PluginContext::new();
        assert!(ctx.config.is_empty());
    }

    #[test]
    fn load_from_dir_nonexistent_returns_error() {
        let mgr = PluginManager::new();
        let result = mgr.load_from_dir("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), PluginError::IoError(_)),
            "expected IoError variant"
        );
    }

    #[test]
    fn plugin_info_clone_and_debug() {
        let info = PluginInfo {
            name: "p".to_owned(),
            version: "1.0".to_owned(),
            author: "me".to_owned(),
            description: "desc".to_owned(),
        };
        let info2 = info.clone();
        assert_eq!(info2.name, "p");
        // Ensure Debug is implemented (no panic on format).
        let _ = format!("{info2:?}");
    }
}
