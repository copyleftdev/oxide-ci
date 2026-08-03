//! WASM plugin host for Oxide CI using Extism.

pub mod host;
pub mod manifest;
pub mod registry;

// New modules
pub mod cache;
pub mod docker;
pub mod git;
pub mod rust_toolchain;
pub mod version;

pub use host::{PluginHost, PluginHostConfig};
pub use manifest::{
    LogEntry, LogLevel, PluginCallInput, PluginCallOutput, PluginInput, PluginManifest,
    PluginOutput, PluginRef,
};
pub use registry::{PluginRegistry, RegistryConfig};
pub use version::{PartialVersion, VersionSpec};

use oxide_core::Result;
use semver::Version;

/// Trait for native plugins.
pub trait Plugin: Send + Sync {
    /// Get the plugin name.
    fn name(&self) -> &str;
    /// The plugin's own semantic version.
    ///
    /// This is what `uses: name@version` is resolved against. Bump the major
    /// when a parameter changes meaning or is removed, the minor when one is
    /// added.
    fn version(&self) -> &str;
    /// Execute the plugin.
    fn execute(&self, input: &PluginCallInput) -> Result<PluginCallOutput>;
}

/// A plugin that ships with the engine.
pub struct BuiltinPlugin {
    /// Canonical name, the one shown to users.
    pub name: &'static str,
    /// Other names that resolve to this plugin, such as GitHub Actions spellings.
    pub aliases: &'static [&'static str],
    make: fn() -> Box<dyn Plugin>,
}

impl BuiltinPlugin {
    /// Instantiate the plugin.
    pub fn instantiate(&self) -> Box<dyn Plugin> {
        (self.make)()
    }

    /// The version this plugin is installed at.
    pub fn version(&self) -> Result<Version> {
        let raw = self.instantiate().version().to_string();
        Version::parse(&raw).map_err(|e| {
            oxide_core::Error::Internal(format!(
                "built-in plugin `{}` declares an invalid version `{raw}`: {e}",
                self.name
            ))
        })
    }

    /// Whether `name` refers to this plugin.
    fn answers_to(&self, name: &str) -> bool {
        self.name == name || self.aliases.contains(&name)
    }
}

/// Every plugin that ships with the engine.
///
/// The single source of truth for what `uses:` can refer to without a registry.
/// User-facing messages are generated from this table so they can never
/// disagree with what [`resolve_builtin`] actually resolves.
pub const BUILTINS: &[BuiltinPlugin] = &[
    BuiltinPlugin {
        name: "git-checkout",
        aliases: &["oxide/checkout"],
        make: || Box::new(git::GitCheckoutPlugin::new()),
    },
    BuiltinPlugin {
        name: "cache",
        aliases: &["oxide/cache"],
        make: || Box::new(cache::CachePlugin::new()),
    },
    BuiltinPlugin {
        name: "docker-build",
        aliases: &["oxide/docker-build"],
        make: || Box::new(docker::DockerBuildPlugin::new()),
    },
    BuiltinPlugin {
        name: "rust-toolchain",
        aliases: &["dtolnay/rust-toolchain"],
        make: || Box::new(rust_toolchain::RustToolchainPlugin::new()),
    },
];

/// A built-in plugin, resolved from a `uses:` reference.
pub struct ResolvedPlugin {
    /// The plugin itself.
    pub plugin: Box<dyn Plugin>,
    /// Canonical name of the plugin that was resolved.
    pub name: &'static str,
    /// The version it is installed at.
    pub version: Version,
    /// Set when the reference carried a suffix that is not a version, such as
    /// `@stable`. The plugin still resolves; the caller should tell the user
    /// the suffix had no effect.
    pub warning: Option<String>,
}

impl std::fmt::Debug for ResolvedPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedPlugin")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("warning", &self.warning)
            .finish_non_exhaustive()
    }
}

/// `name@version` for every built-in, for use in help and error text.
pub fn builtin_summary() -> String {
    BUILTINS
        .iter()
        .map(|builtin| match builtin.version() {
            Ok(version) => format!("{}@{}", builtin.name, version),
            Err(_) => builtin.name.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Resolve a `uses:` reference to a built-in plugin.
///
/// The reference is `name[@version]`, where the version follows the GitHub
/// Actions spelling: `@v1` pins a major, `@v1.2` a minor, `@v1.2.3` an exact
/// release, and an absent suffix or `@latest` takes what is installed. See
/// [`version`] for the full grammar.
///
/// Fails with [`oxide_core::Error::PluginNotFound`] when no built-in answers to
/// the name, and [`oxide_core::Error::PluginVersionMismatch`] when one does but
/// at a version the pipeline did not ask for.
pub fn resolve_builtin(plugin_ref: &str) -> Result<ResolvedPlugin> {
    let reference = PluginRef::parse(plugin_ref);

    let Some(builtin) = BUILTINS
        .iter()
        .find(|builtin| builtin.answers_to(&reference.name))
    else {
        return Err(oxide_core::Error::PluginNotFound(plugin_ref.to_string()));
    };

    let version = builtin.version()?;
    let spec = VersionSpec::parse(reference.version.as_deref());

    if !spec.matches(&version) {
        return Err(oxide_core::Error::PluginVersionMismatch {
            name: builtin.name.to_string(),
            requested: reference.version.unwrap_or_default(),
            available: version.to_string(),
        });
    }

    let warning = match &spec {
        VersionSpec::NotAVersion(raw) => Some(format!(
            "`{}@{raw}` — `{raw}` is not a version, so it was ignored; {} is installed at {version}. \
             Use @v{} to pin the major version.",
            reference.name, builtin.name, version.major
        )),
        _ => None,
    };

    Ok(ResolvedPlugin {
        plugin: builtin.instantiate(),
        name: builtin.name,
        version,
        warning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_builtin_by_bare_name() {
        assert_eq!(
            resolve_builtin("docker-build").unwrap().name,
            "docker-build"
        );
        assert_eq!(
            resolve_builtin("git-checkout").unwrap().name,
            "git-checkout"
        );
    }

    #[test]
    fn resolves_builtin_by_alias() {
        assert_eq!(resolve_builtin("oxide/cache").unwrap().name, "cache");
        assert_eq!(
            resolve_builtin("dtolnay/rust-toolchain").unwrap().name,
            "rust-toolchain"
        );
    }

    #[test]
    fn resolves_matching_version_suffix() {
        // Regression: #50 — `uses: docker-build@v1` reported "Plugin not found"
        // while the same message advertised docker-build as supported.
        let resolved = resolve_builtin("docker-build@v1").unwrap();
        assert_eq!(resolved.name, "docker-build");
        assert_eq!(resolved.version.major, 1);
        assert!(resolved.warning.is_none());

        assert!(resolve_builtin("docker-build@1").is_ok());
        assert!(resolve_builtin("docker-build@1.0").is_ok());
        assert!(resolve_builtin("docker-build@1.0.0").is_ok());
        assert!(resolve_builtin("docker-build@latest").is_ok());
        assert!(resolve_builtin("oxide/checkout@v1").is_ok());
    }

    #[test]
    fn rejects_version_that_is_not_installed() {
        let err = resolve_builtin("docker-build@v99").unwrap_err();
        assert!(
            matches!(err, oxide_core::Error::PluginVersionMismatch { .. }),
            "expected a version mismatch, got {err:?}"
        );
        // The message has to name what was asked for and what exists.
        let message = err.to_string();
        assert!(message.contains("docker-build"), "{message}");
        assert!(message.contains("v99"), "{message}");
        assert!(message.contains("1.0.0"), "{message}");

        assert!(resolve_builtin("docker-build@2").is_err());
        assert!(resolve_builtin("docker-build@1.9").is_err());
        assert!(resolve_builtin("docker-build@1.0.7").is_err());
    }

    #[test]
    fn non_version_suffix_resolves_with_a_warning() {
        // `dtolnay/rust-toolchain@stable` is a branch ref, not a version.
        // Pipelines copied from GitHub Actions keep working, but the user is
        // told the suffix did nothing.
        let resolved = resolve_builtin("dtolnay/rust-toolchain@stable").unwrap();
        assert_eq!(resolved.name, "rust-toolchain");
        let warning = resolved.warning.expect("expected a warning");
        assert!(warning.contains("stable"), "{warning}");
        assert!(warning.contains("not a version"), "{warning}");
    }

    #[test]
    fn unknown_plugin_does_not_resolve() {
        for reference in ["does-not-exist", "does-not-exist@v1", "@v1", ""] {
            assert!(
                matches!(
                    resolve_builtin(reference),
                    Err(oxide_core::Error::PluginNotFound(_))
                ),
                "`{reference}` should not resolve"
            );
        }
    }

    #[test]
    fn every_builtin_declares_a_valid_version() {
        for builtin in BUILTINS {
            builtin
                .version()
                .unwrap_or_else(|e| panic!("{}: {e}", builtin.name));
        }
    }

    #[test]
    fn every_builtin_resolves_by_name_alias_and_major_pin() {
        // Keeps the user-facing plugin list honest: everything the CLI
        // advertises must resolve, bare, by alias, and pinned to its major.
        for builtin in BUILTINS {
            let version = builtin.version().unwrap();
            assert!(resolve_builtin(builtin.name).is_ok(), "{}", builtin.name);
            assert!(
                resolve_builtin(&format!("{}@v{}", builtin.name, version.major)).is_ok(),
                "{}@v{}",
                builtin.name,
                version.major
            );
            for alias in builtin.aliases {
                assert_eq!(
                    resolve_builtin(alias).unwrap().name,
                    builtin.name,
                    "alias `{alias}`"
                );
            }
        }
    }

    #[test]
    fn summary_lists_every_builtin_with_its_version() {
        let summary = builtin_summary();
        for builtin in BUILTINS {
            let version = builtin.version().unwrap();
            assert!(
                summary.contains(&format!("{}@{version}", builtin.name)),
                "`{summary}` is missing {}",
                builtin.name
            );
        }
    }

    #[test]
    fn plugin_name_matches_its_registry_entry() {
        for builtin in BUILTINS {
            assert_eq!(
                builtin.instantiate().name(),
                builtin.name,
                "registry name and Plugin::name disagree"
            );
        }
    }
}
