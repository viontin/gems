//! Inertia middleware — renders JSON for XHR, HTML for full page loads.

use serde_json::{json, Value};
use viontin_framework::http::{Request, Response};
use viontin_framework::middleware::{Middleware, Next};

/// Middleware that powers the InertiaJS server-side protocol.
///
/// On XHR requests (`X-Inertia: true`): passes the JSON page through.
/// On full page loads: renders the root view HTML with embedded `data-page` JSON.
/// On redirects: returns 409 with `X-Inertia-Location` per Inertia protocol.
#[derive(Debug)]
pub struct InertiaMiddleware;

impl InertiaMiddleware {
    pub fn new() -> Self { InertiaMiddleware }

    pub fn is_inertia(req: &Request) -> bool {
        req.header("x-inertia").is_some()
    }

    pub fn partial_data(req: &Request) -> Vec<String> {
        req.header("x-inertia-partial-data")
            .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
            .unwrap_or_default()
    }

    /// Extract the `data-page` JSON from an Inertia response body.
    /// Returns `None` if the body is not an Inertia page JSON.
    fn parse_page_data(body: &str) -> Option<Value> {
        serde_json::from_str::<Value>(body).ok().filter(|v| v.get("component").is_some())
    }

    /// Build a full HTML page with the Inertia data embedded.
    fn render_html(page_json: &str, template: &str) -> String {
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

        // Run the handler
        let mut response = next(req);

        // ── Handle redirects per Inertia protocol ──
        if response.status.0 >= 300 && response.status.0 < 400 {
            if let Some(location) = response.headers.get("location").map(|s| s.to_string()) {
                if is_inertia {
                    let payload = json!({
                        "component": null, "props": {}, "url": location, "version": version,
                    });
                    return Response::json(&payload)
                        .unwrap_or(Response::html("{}"))
                        .with_header("X-Inertia-Location", &location)
                        .with_header("X-Inertia-Version", &version);
                }
                return response;
            }
        }

        // ── Build page payload ──
        let body_str = response.body_str().to_string();
        let page_data = match Self::parse_page_data(&body_str) {
            Some(data) => {
                // Handler returned an InertiaPage — use its data
                let mut map = match data {
                    Value::Object(m) => m,
                    _ => serde_json::Map::new(),
                };
                // Merge shared props
                let shared = super::shared();
                for (k, v) in shared { map.insert(k, v); }

                // Filter partial reload keys
                if !partial_keys.is_empty() {
                    if let Some(Value::Object(props)) = map.get("props") {
                        let filtered: serde_json::Map<String, Value> = props
                            .iter()
                            .filter(|(k, _)| partial_keys.contains(k))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        map.insert("props".into(), Value::Object(filtered));
                    }
                }
                map.insert("version".into(), Value::String(version.clone()));
                Value::Object(map)
            }
            None => {
                // Handler returned a plain response — wrap it
                json!({
                    "component": "Page",
                    "props": { "body": body_str },
                    "url": url,
                    "version": version,
                })
            }
        };

        let page_json = page_data.to_string();

        // ── Respond ──
        if is_inertia {
            Response::text(&page_json)
                .with_header("content-type", "application/json")
                .with_header("X-Inertia-Version", &version)
        } else {
            // Full page load: embed data-page in root view HTML
            let template = super::root_view();
            match template {
                Some(ref root) => {
                    let html = Self::render_html(&page_json, root);
                    Response::html(&html)
                }
                None => {
                    // Fallback: return raw JSON if no template configured
                    response.headers.set("content-type", "application/json");
                    response.body = page_json.into_bytes();
                    response
                }
            }
        }
    }
}
