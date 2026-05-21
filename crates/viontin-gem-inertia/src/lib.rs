//! Viontin Inertia Gem — InertiaJS-compatible server adapter.
//!
//! Implements the InertiaJS server-side protocol:
//! - Detects `X-Inertia` header → returns JSON `{ component, props, url, version }`
//! - Full page load → renders HTML with embedded `data-page` JSON
//! - Redirect via 409 Conflict + `X-Inertia-Location`
//! - Partial reloads via `X-Inertia-Partial-*` headers
//!
//! Usage in Viontin (middleware auto-wired via GemBinding):
//! ```rust
//! use viontin_gem_inertia::{Inertia, inertia, share};
//!
//! fn main() {
//!     viontin::boot()
//!         .gem(Inertia::load().entry("resources/views/app.html"))
//!         .get("/", |_| inertia("Home", json!({ "title": "Welcome" })))
//!         .get("/users", |_| inertia("Users/Index", json!({ "users": users })))
//!         .serve(":3000");
//! }
//! ```

mod middleware;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use serde_json::Value;

pub use middleware::InertiaMiddleware;

// ── Gem Facade ──

use viontin_gems::{GemBuilder, GemMeta, GemKind, GemFacade, GemBinding};
use viontin_framework::middleware::Middleware;
use viontin_framework::Result;

pub const META: GemMeta = GemMeta::new(
    "inertia",
    "0.1.0",
    "InertiaJS server adapter",
    GemKind::Integration,
);

#[derive(Debug)]
pub struct Inertia {
    root_view: String,
}

impl Inertia {
    /// Entry point for the root view template.
    ///
    /// The template should contain `{{data-page}}` where the page JSON will be injected.
    pub fn entry(mut self, root_view: &str) -> Self {
        self.root_view = root_view.into();
        self
    }

    /// Middleware instance (used internally by GemBinding).
    pub fn middleware() -> InertiaMiddleware {
        InertiaMiddleware::new()
    }
}

impl GemBuilder for Inertia {
    fn load() -> Self {
        Inertia { root_view: String::new() }
    }
}

impl GemFacade for Inertia {
    fn meta(&self) -> &GemMeta { &META }

    fn before_build(&self) -> Result<()> {
        // Verify the root view exists if it's a file path
        let path = std::path::Path::new(&self.root_view);
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => { set_root_view(&content); }
                Err(e) => { eprintln!("  [inertia] Warning: cannot read root view: {}", e); }
            }
        } else {
            set_root_view(&self.root_view);
        }
        println!("  [inertia] Ready (root view: {})", self.root_view);
        Ok(())
    }
}

impl GemBinding for Inertia {
    fn gem_middlewares(&self) -> Vec<Box<dyn Middleware + 'static>> {
        vec![Box::new(InertiaMiddleware::new())]
    }
}

// ── Global State ──

struct InertiaConfig {
    version: String,
    root_view: Option<String>,
    shared: HashMap<String, Value>,
}

impl InertiaConfig {
    fn new() -> Self {
        InertiaConfig {
            version: "1.0".into(),
            root_view: None,
            shared: HashMap::new(),
        }
    }
}

static CONFIG: OnceLock<Mutex<InertiaConfig>> = OnceLock::new();

fn config() -> &'static Mutex<InertiaConfig> {
    CONFIG.get_or_init(|| Mutex::new(InertiaConfig::new()))
}

// ── Public API ──

/// Set the asset version. Client will full reload on mismatch.
pub fn set_version(version: &str) {
    if let Ok(mut c) = config().lock() { c.version = version.into(); }
}

/// Get the current asset version.
pub fn version() -> String {
    config().lock().map(|c| c.version.clone()).unwrap_or_default()
}

/// Set the root view HTML template content.
pub fn set_root_view(html: &str) {
    if let Ok(mut c) = config().lock() { c.root_view = Some(html.into()); }
}

/// Get the root view HTML template.
pub fn root_view() -> Option<String> {
    config().lock().ok().and_then(|c| c.root_view.clone())
}

/// Share global props (available on every page).
pub fn share(key: &str, value: Value) {
    if let Ok(mut c) = config().lock() { c.shared.insert(key.into(), value); }
}

/// Unshare a global prop.
pub fn unshare(key: &str) {
    if let Ok(mut c) = config().lock() { c.shared.remove(key); }
}

/// Get all shared props.
pub fn shared() -> HashMap<String, Value> {
    config().lock().map(|c| c.shared.clone()).unwrap_or_default()
}

/// Flush all shared props.
pub fn flush_shared() {
    if let Ok(mut c) = config().lock() { c.shared.clear(); }
}

// ── Response Builder ──

/// Create an Inertia page response.
///
/// ```rust
/// inertia("User/Profile", json!({ "id": 42, "name": "Alice" }))
/// ```
pub fn inertia(component: &str, props: Value) -> InertiaPage {
    InertiaPage {
        component: component.into(),
        props,
        url: String::new(),
        status_code: 200,
        redirect_url: None,
        partial_component: None,
        partial_data: Vec::new(),
    }
}

/// An Inertia page response.
#[derive(Debug, Clone)]
pub struct InertiaPage {
    component: String,
    props: Value,
    url: String,
    status_code: u16,
    redirect_url: Option<String>,
    partial_component: Option<String>,
    partial_data: Vec<String>,
}

impl InertiaPage {
    /// Set the URL manually (overrides auto-detection).
    pub fn url(mut self, url: &str) -> Self { self.url = url.into(); self }

    /// Set the HTTP status code.
    pub fn status(mut self, code: u16) -> Self { self.status_code = code; self }

    /// Merge additional props.
    pub fn with(mut self, props: Value) -> Self {
        if let Value::Object(ref mut map) = self.props {
            if let Value::Object(extra) = props { map.extend(extra); }
        }
        self
    }

    /// Mark as partial reload (only these props are returned).
    pub fn only(mut self, keys: &[&str]) -> Self {
        self.partial_data = keys.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Create a redirect response.
    pub fn redirect(url: &str) -> Self {
        InertiaPage {
            component: String::new(),
            props: Value::Null,
            url: url.into(),
            status_code: 303,
            redirect_url: Some(url.into()),
            partial_component: None,
            partial_data: Vec::new(),
        }
    }

    /// Redirect back.
    pub fn back() -> Self {
        InertiaPage::redirect("/")
    }
}

// ── Convert to HTTP Response ──

use viontin_framework::http::Response;

impl From<InertiaPage> for Response {
    fn from(page: InertiaPage) -> Self {
        if let Some(url) = &page.redirect_url {
            return Response::new(viontin_framework::http::StatusCode::FOUND)
                .with_header("Location", url.as_str());
        }

        let data = serde_json::json!({
            "component": page.component,
            "props": page.props,
            "url": page.url,
            "version": version(),
        });

        Response::json(&data).unwrap_or_else(|_| Response::html("{}"))
    }
}
