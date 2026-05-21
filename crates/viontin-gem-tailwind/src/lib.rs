pub mod config;
pub mod compiler;

use std::path::Path;
use viontin_framework::Result;
use viontin_framework::error::FrameworkError;
use viontin_framework::gem::{GemMeta, GemKind, GemFacade};

pub const META: GemMeta = GemMeta::new(
    "tailwind",
    "0.1.0",
    "TailwindCSS build-time CSS generation",
    GemKind::Theme,
);

#[derive(Debug)]
pub struct Gem;

impl GemFacade for Gem {
    fn meta(&self) -> &GemMeta { &META }

    fn before_build(&self) -> Result<()> {
        let project_root = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => { eprintln!("  [tailwind] Error: {}", e); return Ok(()); }
        };
        let cfg = config::TailwindConfig::load(&project_root.join("tailwind.config.toml"));
        let css = match compiler::compile_project(&project_root) {
            Ok(c) => c,
            Err(e) => { eprintln!("  [tailwind] Error: {}", e); return Ok(()); }
        };
        let output_path = project_root.join(&cfg.output_path);
        if let Err(e) = compiler::write_css(&css, &output_path) {
            eprintln!("  [tailwind] Error: {}", e);
            return Ok(());
        }
        println!("  [tailwind] CSS generated ({} bytes)", css.len());
        Ok(())
    }
}

pub fn compile_project(project_root: &Path) -> Result<String> {
    let css = compiler::compile_project(project_root)
        .map_err(|e| FrameworkError::Internal(e))?;
    let cfg = config::TailwindConfig::load(&project_root.join("tailwind.config.toml"));
    compiler::write_css(&css, &project_root.join(&cfg.output_path))
        .map_err(|e| FrameworkError::Internal(e))?;
    Ok(css)
}

pub fn compile_source(content: &str) -> Result<String> {
    compiler::compile_source(content)
        .map_err(|e| FrameworkError::Internal(e))
}
