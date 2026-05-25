use std::path::Path;
use std::fs;
use std::collections::HashSet;
use tailwind_rs_core::CssGenerator;

pub fn compile_project(project_root: &Path) -> Result<String, String> {
    let content_patterns = vec![
        "src/**/*.rs".to_string(),
        "src/**/*.html".to_string(),
        "html/**/*.html".to_string(),
    ];

    let files = collect_content_files(&content_patterns, project_root);
    if files.is_empty() {
        return Err("No content files found".into());
    }

    let mut all_classes = HashSet::new();
    for file in &files {
        let content = fs::read_to_string(file)
            .map_err(|e| format!("Cannot read {}: {}", file.display(), e))?;
        for class in extract_classes(&content) {
            all_classes.insert(class);
        }
    }

    let mut generator = CssGenerator::new();
    for class in &all_classes {
        if let Err(e) = generator.add_class(class) {
            eprintln!("  [tailwind] skipping '{}': {}", class, e);
        }
    }

    let css = generator.generate_css();
    Ok(css)
}

pub fn compile_source(content: &str) -> Result<String, String> {
    let mut generator = CssGenerator::new();
    for class in extract_classes(&content) {
        generator.add_class(&class).ok();
    }
    Ok(generator.generate_css())
}

fn extract_classes(content: &str) -> HashSet<String> {
    let mut classes = HashSet::new();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 7 <= len && bytes[i..i+7].eq_ignore_ascii_case(b"class=\"") {
            i += 7;
            let start = i;
            while i < len && bytes[i] != b'"' { i += 1; }
            if i > start {
                if let Ok(s) = std::str::from_utf8(&bytes[start..i]) {
                    for token in s.split_whitespace() {
                        if !token.is_empty() { classes.insert(token.to_string()); }
                    }
                }
            }
            i += 1;
            continue;
        }

        if i + 7 <= len && bytes[i..i+7].eq_ignore_ascii_case(b"class='") {
            i += 7;
            let start = i;
            while i < len && bytes[i] != b'\'' { i += 1; }
            if i > start {
                if let Ok(s) = std::str::from_utf8(&bytes[start..i]) {
                    for token in s.split_whitespace() {
                        if !token.is_empty() { classes.insert(token.to_string()); }
                    }
                }
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    classes
}

fn collect_content_files(patterns: &[String], project_root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for pattern in patterns {
        let full_pattern = if pattern.starts_with('/') {
            pattern.clone()
        } else {
            format!("{}/{}", project_root.to_string_lossy(), pattern)
        };
        if let Ok(paths) = glob::glob(&full_pattern) {
            for entry in paths.flatten() {
                if entry.is_file() {
                    files.push(entry);
                }
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

pub fn write_css(css: &str, output_path: &Path) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }
    fs::write(output_path, css)
        .map_err(|e| format!("Failed to write CSS: {}", e))?;
    Ok(())
}
