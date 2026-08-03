//! Version requirements for plugin references.
//!
//! A plugin reference is `name[@version]`. The version half follows the
//! GitHub Actions spelling users already know: `@v1` pins a major version,
//! `@v1.2` a minor, `@v1.2.3` an exact release, and an absent suffix or
//! `@latest` accepts whatever is installed.
//!
//! Deliberately *not* cargo's caret semantics: `@1.0.0` here means exactly
//! 1.0.0, because a pipeline that pins three components is asking for one
//! release, not a range.

use semver::Version;
use std::fmt;

/// What the `@…` suffix on a plugin reference asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionSpec {
    /// No suffix, or `@latest`: any version satisfies it.
    Any,
    /// A version pin of one, two, or three components.
    Pin(PartialVersion),
    /// A suffix that is not a version at all — a git branch or toolchain
    /// channel such as `@stable` or `@main`. Kept rather than discarded so
    /// callers can tell the user it had no effect.
    NotAVersion(String),
}

/// A version pin with an optional minor and patch, e.g. `v1`, `1.2`, `1.2.3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialVersion {
    pub major: u64,
    pub minor: Option<u64>,
    pub patch: Option<u64>,
}

impl VersionSpec {
    /// Parse the version half of a plugin reference.
    pub fn parse(raw: Option<&str>) -> Self {
        let Some(raw) = raw else {
            return Self::Any;
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("latest") {
            return Self::Any;
        }

        // Accept the `v` prefix GitHub Actions users type.
        let digits = trimmed
            .strip_prefix('v')
            .or_else(|| trimmed.strip_prefix('V'))
            .unwrap_or(trimmed);

        match PartialVersion::parse(digits) {
            Some(pin) => Self::Pin(pin),
            None => Self::NotAVersion(trimmed.to_string()),
        }
    }

    /// Whether an installed version satisfies this spec.
    ///
    /// A non-version suffix satisfies everything: it is reported to the user as
    /// a warning instead of failing a pipeline over a spelling we cannot honor.
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Any | Self::NotAVersion(_) => true,
            Self::Pin(pin) => pin.matches(version),
        }
    }
}

impl PartialVersion {
    fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('.');
        let major = parse_component(parts.next()?)?;
        let minor = match parts.next() {
            Some(part) => Some(parse_component(part)?),
            None => None,
        };
        let patch = match parts.next() {
            Some(part) => Some(parse_component(part)?),
            None => None,
        };
        if parts.next().is_some() {
            return None; // more than three components is not a version
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    fn matches(&self, version: &Version) -> bool {
        self.major == version.major
            && self.minor.is_none_or(|m| m == version.minor)
            && self.patch.is_none_or(|p| p == version.patch)
    }
}

fn parse_component(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

impl fmt::Display for PartialVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.major)?;
        if let Some(minor) = self.minor {
            write!(f, ".{minor}")?;
        }
        if let Some(patch) = self.patch {
            write!(f, ".{patch}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn absent_and_latest_accept_anything() {
        assert_eq!(VersionSpec::parse(None), VersionSpec::Any);
        assert_eq!(VersionSpec::parse(Some("")), VersionSpec::Any);
        assert_eq!(VersionSpec::parse(Some("latest")), VersionSpec::Any);
        assert_eq!(VersionSpec::parse(Some("LATEST")), VersionSpec::Any);
        assert!(VersionSpec::parse(None).matches(&v("7.3.1")));
    }

    #[test]
    fn major_pin_matches_any_minor_and_patch() {
        let spec = VersionSpec::parse(Some("v1"));
        assert!(spec.matches(&v("1.0.0")));
        assert!(spec.matches(&v("1.9.4")));
        assert!(!spec.matches(&v("2.0.0")));
        assert!(!spec.matches(&v("0.9.0")));
    }

    #[test]
    fn v_prefix_is_optional() {
        assert_eq!(
            VersionSpec::parse(Some("v1")),
            VersionSpec::parse(Some("1"))
        );
        assert!(VersionSpec::parse(Some("1")).matches(&v("1.2.3")));
    }

    #[test]
    fn minor_pin_matches_any_patch() {
        let spec = VersionSpec::parse(Some("v1.2"));
        assert!(spec.matches(&v("1.2.0")));
        assert!(spec.matches(&v("1.2.9")));
        assert!(!spec.matches(&v("1.3.0")));
    }

    #[test]
    fn exact_pin_is_exact() {
        // Not caret semantics: three components means one release.
        let spec = VersionSpec::parse(Some("1.0.0"));
        assert!(spec.matches(&v("1.0.0")));
        assert!(!spec.matches(&v("1.0.1")));
        assert!(!spec.matches(&v("1.2.3")));
    }

    #[test]
    fn mismatched_major_does_not_match() {
        assert!(!VersionSpec::parse(Some("v99")).matches(&v("1.0.0")));
    }

    #[test]
    fn branch_and_channel_refs_are_not_versions() {
        for raw in ["stable", "beta", "nightly", "main", "master", "my-branch"] {
            assert_eq!(
                VersionSpec::parse(Some(raw)),
                VersionSpec::NotAVersion(raw.to_string()),
                "`{raw}` should not parse as a version"
            );
        }
        // They still resolve, so pipelines copied from GitHub Actions run.
        assert!(VersionSpec::parse(Some("stable")).matches(&v("1.0.0")));
    }

    #[test]
    fn malformed_versions_are_not_versions() {
        for raw in ["1.", "1.2.3.4", "1.x", "v", "1..2", "-1"] {
            assert!(
                matches!(VersionSpec::parse(Some(raw)), VersionSpec::NotAVersion(_)),
                "`{raw}` should not parse as a version"
            );
        }
    }

    #[test]
    fn pin_displays_as_written() {
        let pin = |s: &str| match VersionSpec::parse(Some(s)) {
            VersionSpec::Pin(p) => p.to_string(),
            other => panic!("expected a pin, got {other:?}"),
        };
        assert_eq!(pin("v1"), "1");
        assert_eq!(pin("1.2"), "1.2");
        assert_eq!(pin("1.2.3"), "1.2.3");
    }
}
