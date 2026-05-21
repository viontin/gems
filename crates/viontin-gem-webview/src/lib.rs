//! Viontin Webview Gem — desktop webview integration via wry + tao.
//!
//! Opens a native desktop window with a webview pointing to a Viontin HTTP server,
//! enabling desktop applications with HTML/CSS/JS frontends.
//!
//! Usage:
//! ```rust
//! use viontin::boot;
//! use viontin_gem_webview::WebviewGem;
//!
//! fn main() {
//!     boot()
//!         .gem(WebviewGem::load()
//!             .title("My App")
//!             .size(1024, 768)
//!         )
//!         .get("/", |_| Response::html("<h1>Hello from Desktop!</h1>"))
//!         .entry(|ctx| {
//!             viontin_gem_webview::launch(ctx.ws_server, ":3000");
//!         })
//!         .run();
//! }
//! ```

use std::sync::{Mutex, OnceLock};
use viontin_framework::Result;
use viontin_framework::ws::WsServer;
use viontin_gems::{GemBuilder, GemMeta, GemKind, GemFacade, GemBinding};

pub const META: GemMeta = GemMeta::new(
    "webview",
    "0.1.0",
    "Desktop webview integration (wry + tao)",
    GemKind::Integration,
);

// ── Config ──

#[derive(Clone)]
struct WebviewConfig {
    title: String,
    width: u32,
    height: u32,
    devtools: bool,
}

impl Default for WebviewConfig {
    fn default() -> Self {
        WebviewConfig {
            title: "Viontin App".into(),
            width: 1024,
            height: 768,
            devtools: false,
        }
    }
}

static CONFIG: OnceLock<Mutex<WebviewConfig>> = OnceLock::new();

fn config() -> &'static Mutex<WebviewConfig> {
    CONFIG.get_or_init(|| Mutex::new(WebviewConfig::default()))
}

// ── Gem ──

#[derive(Debug)]
pub struct WebviewGem {
    config: WebviewConfig,
}

impl WebviewGem {
    /// Set the window title.
    pub fn title(mut self, title: &str) -> Self {
        self.config.title = title.into();
        self
    }

    /// Set the initial window size (width, height).
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    /// Enable or disable devtools (default: false).
    pub fn devtools(mut self, enabled: bool) -> Self {
        self.config.devtools = enabled;
        self
    }
}

impl GemBuilder for WebviewGem {
    fn load() -> Self {
        WebviewGem {
            config: WebviewConfig::default(),
        }
    }
}

impl GemFacade for WebviewGem {
    fn meta(&self) -> &GemMeta {
        &META
    }

    fn before_build(&self) -> Result<()> {
        let mut cfg = config().lock().unwrap();
        cfg.title = self.config.title.clone();
        cfg.width = self.config.width;
        cfg.height = self.config.height;
        cfg.devtools = self.config.devtools;
        println!("  [webview] Ready ({}x{}, devtools: {})", cfg.width, cfg.height, cfg.devtools);
        Ok(())
    }
}

impl GemBinding for WebviewGem {}

// ── Launch ──

/// Launch the desktop window with an embedded HTTP server.
///
/// - Starts the HTTP server in a background thread.
/// - Opens a native window with a webview pointed at `http://127.0.0.1:{port}`.
/// - Exits the process when the window is closed.
///
/// The `addr` format follows Viontin's server convention: `":3000"` or `"127.0.0.1:3000"`.
pub fn launch(ws_server: WsServer, addr: &str) {
    let cfg = config().lock().unwrap().clone();
    let addr = addr.to_owned();
    let url = format!("http://127.0.0.1:{}", addr.trim_start_matches(':'));

    std::thread::spawn(move || {
        if let Err(e) = ws_server.run(&addr) {
            eprintln!("  [webview] Server error: {}", e);
        }
    });

    let event_loop = tao::event_loop::EventLoop::new();
    let window = tao::window::WindowBuilder::new()
        .with_title(&cfg.title)
        .with_inner_size(tao::dpi::LogicalSize::new(
            cfg.width as f64,
            cfg.height as f64,
        ))
        .build(&event_loop)
        .expect("[webview] Failed to create window");

    let _webview = wry::WebViewBuilder::new()
        .with_url(&url)
        .with_devtools(cfg.devtools)
        .build(&window)
        .expect("[webview] Failed to create webview");

    println!("  [webview] Desktop window open — {}", url);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = tao::event_loop::ControlFlow::Wait;

        if let tao::event::Event::WindowEvent {
            event: tao::event::WindowEvent::CloseRequested,
            ..
        } = event {
            *control_flow = tao::event_loop::ControlFlow::Exit;
        }
    });
}
