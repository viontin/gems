use viontin_framework::validator::{Validator, Context, Outcome};
use viontin_framework::semver::{Version, VersionReq, Meta};

/// Validates gem metadata — name, version, kind, configuration.
///
/// Ensures every gem follows uniform rules before registration.
pub fn validate_gem_meta(
    name: &str,
    version: &Version,
    kind: &str,
    homepage: Option<&str>,
) -> Outcome {
    let mut result = Outcome::new();

    // Name must not be empty
    if name.is_empty() {
        result.error("G001", "Gem name must not be empty");
    } else if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        result.error("G002", format!("Gem name '{}' contains invalid characters", name));
    }

    // Version must satisfy framework compatibility
    let fw = Meta::current();
    let framework_req = VersionReq::parse(&format!("^{}.{}.{}", fw.version.major, fw.version.minor, fw.version.patch))
        .unwrap_or_else(|_| VersionReq::Any);

    if !framework_req.matches(version) {
        result.warning("G003", format!(
            "Gem version {} may not be compatible with framework {}",
            version, fw.version
        ));
    }

    // Kind must be a valid gem category
    let valid_kinds = ["codegen", "platform", "integration", "database", "auth",
                        "devtool", "theme", "deploy", "analytics", "security",
                        "storage", "notification", "lsp"];
    if !valid_kinds.contains(&kind) {
        result.warning("G004", format!("Unknown gem kind '{}' — using custom", kind));
    }

    // Homepage should be a URL if provided
    if let Some(url) = homepage {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            result.warning("G005", format!("Gem homepage '{}' is not a valid URL", url));
        }
    }

    result
}

/// Validates that a gem's declared targets are valid.
pub fn validate_gem_targets(supports: &[&str], defaults: &[&str]) -> Outcome {
    let mut result = Outcome::new();

    let valid_targets = ["hybrid", "ssr", "csr", "static", "api", "desktop", "mobile"];

    for target in supports {
        if !valid_targets.contains(target) {
            result.warning("G010", format!("Unknown target '{}' in supports list", target));
        }
    }

    for target in defaults {
        if !supports.contains(target) {
            result.error("G011", format!(
                "Gem declares default_for '{}' but does not list it in supports", target
            ));
        }
    }

    result
}

/// Validates gem registration — ensures no name collisions.
pub fn validate_gem_registration(
    existing_names: &[String],
    new_name: &str,
) -> Outcome {
    let mut result = Outcome::new();

    if existing_names.contains(&new_name.to_string()) {
        result.error("G020", format!("Gem '{}' is already registered", new_name));
    }

    result
}

/// A validator that wraps gem validation into the framework Validator trait.
#[derive(Debug)]
pub struct GemMetaValidator {
    pub gem_name: String,
    pub gem_version: Version,
    pub gem_kind: String,
}

impl Validator for GemMetaValidator {
    fn name(&self) -> &str { "gem-meta" }

    fn validate(&self, _ctx: &Context) -> Outcome {
        validate_gem_meta(&self.gem_name, &self.gem_version, &self.gem_kind, None)
    }
}
