use std::path::Path;

pub struct TailwindConfig {
    pub output_path: String,
}

impl Default for TailwindConfig {
    fn default() -> Self {
        TailwindConfig {
            output_path: "assets/tailwind.css".to_string(),
        }
    }
}

impl TailwindConfig {
    pub fn load(path: &Path) -> Self {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = toml::from_str::<TomlConfig>(&content) {
                    if let Some(output) = config.output {
                        return TailwindConfig {
                            output_path: output.path.unwrap_or_else(|| "assets/tailwind.css".to_string()),
                        };
                    }
                }
            }
        }
        TailwindConfig::default()
    }
}

#[derive(serde::Deserialize)]
struct TomlConfig {
    output: Option<TomlOutput>,
}

#[derive(serde::Deserialize)]
struct TomlOutput {
    path: Option<String>,
}
