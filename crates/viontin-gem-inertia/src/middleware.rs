//! Inertia middleware — detects X-Inertia header and renders JSON or HTML.

use serde_json::{json, Value};
use viontin_framework::http::{Request, Response, StatusCode};
use viontin_framework::middleware::{Middleware, Next};

/// Middleware that powers InertiaJS server-side protocol.
///
/// Behavior:
/// - `X-Inertia` header present → JSON response
/// - No header → Full HTML with embedded `data-page`
/// - Redirect → 409 Conflict with `X-Inertia-Location` (Inertia protocol)
#[derive(Debug)]
pub struct InertiaMiddleware {
    root_view: Option<String>,
}

impl InertiaMiddleware {
    pub fn new() -> Self {
        InertiaMiddleware { root_view: None }
    }

    /// Set the root view HTML template.
    pub fn with_root_view(mut self, template: &str) -> Self {
        self.root_view = Some(template.into());
        self
    }

    /// Check if request is an Inertia XHR.
    pub fn is_inertia(req: &Request) -> bool {
        req.header("x-inertia").is_some()
    }

    /// Get the partial component name (for partial reloads).
    pub fn partial_component(req: &Request) -> Option<String> {
        req.header("x-inertia-partial-component").map(|s| s.to_string())
    }

    /// Get the partial data keys (for partial reloads).
    pub fn partial_data(req: &Request) -> Vec<String> {
        req.header("x-inertia-partial-data")
            .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
            .unwrap_or_default()
    }

    /// Build the full page JSON with shared props.
    pub fn page_json(component: &str, props: Value, url: &str, partial_keys: &[String]) -> Value {
        let mut all_props: serde_json::Map<String, Value> = match props {
            Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };

        // Merge shared props
        let shared = super::shared();
        for (k, v) in shared { all_props.insert(k, v); }

        let final_props = if !partial_keys.is_empty() {
            let filtered: serde_json::Map<String, Value> = all_props
                .into_iter()
                .filter(|(k, _)| partial_keys.contains(k))
                .collect();
            Value::Object(filtered)
        } else {
            Value::Object(all_props)
        };

        json!({
            "component": component,
            "props": final_props,
            "url": url,
            "version": super::version(),
        })
    }

    /// Wrap page JSON in the root view HTML template.
    pub fn wrap_html(page_json: &str, template: &str) -> String {
        template.replace("{{data-page}}", page_json)
    }
}

impl Default for InertiaMiddleware {
    fn default() -> Self { Self::new() }
}

impl Middleware for InertiaMiddleware {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        let is_inertia = Self::is_inertia(req);
        let partial_keys = Self::partial_data(req);
        let url = req.uri.path.clone();
        let version = super::version();

        // Execute the handler
        let response = next(req);

        // Handle redirects per Inertia protocol (409 Conflict)
        if response.status.0 >= 300 && response.status.0 < 400 {
            if let Some(location) = response.headers.get("location").map(|s| s.to_string()) {
                if is_inertia {
                    let payload = json!({
                        "component": null,
                        "props": {},
                        "url": location,
                        "version": version,
                    });
                    return Response::json(&payload)
                        .unwrap_or(Response::html("{}"))
                        .with_header("X-Inertia-Location", &location)
                        .with_header("X-Inertia-Version", &version);
                }
                // Full page: just pass through the redirect
                return response;
            }
        }

        // Build page data from the handler's response
        let body_str = response.body_str().to_string();

        // Try to parse the body as InertiaPage JSON
        let page_data = if is_inertia {
            // For XHR: build JSON payload
            let component = "Page";
            Self::page_json(component, json!({ "body": body_str }), &url, &partial_keys)
        } else {
            // For HTML: use body directly
            json!({
                "component": "Page",
                "props": {},
                "url": url,
                "version": version,
            })
        };

        if is_inertia {
            Response::json(&page_data)
                .unwrap_or(response)
                .with_header("X-Inertia-Version", &version)
        } else {
            // Full page load: render HTML with embedded data-page
            let global_root = super::root_view();
            let root_template = self.root_view.as_ref().or_else(|| global_root.as_ref());
            if let Some(ref root) = root_template {
                let html = Self::wrap_html(&page_data.to_string(), root);
                Response::html(&html)
            } else {
                response
            }
        }
    }
}
