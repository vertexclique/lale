use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowFacts {
    pub loop_bounds: HashMap<String, LoopBound>,
    pub entry_point: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopBound {
    pub location: String,
    pub min_iterations: u32,
    pub max_iterations: u32,
}

impl FlowFacts {
    pub fn new() -> Self {
        Self {
            loop_bounds: HashMap::new(),
            entry_point: None,
        }
    }

    /// Parse flow facts from C source file
    pub fn parse_from_source(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read source file")?;

        let mut facts = FlowFacts::new();

        // Regex for loopbound pragma: _Pragma( "loopbound min X max Y" )
        let loopbound_re =
            Regex::new(r#"_Pragma\s*\(\s*"loopbound\s+min\s+(\d+)\s+max\s+(\d+)"\s*\)"#).unwrap();

        // Regex for entrypoint pragma: _Pragma( "entrypoint" )
        let entrypoint_re = Regex::new(r#"_Pragma\s*\(\s*"entrypoint"\s*\)"#).unwrap();

        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            // Check for loopbound pragma
            if let Some(caps) = loopbound_re.captures(line) {
                let min = caps
                    .get(1)
                    .unwrap()
                    .as_str()
                    .parse::<u32>()
                    .context("Failed to parse min iterations")?;
                let max = caps
                    .get(2)
                    .unwrap()
                    .as_str()
                    .parse::<u32>()
                    .context("Failed to parse max iterations")?;

                // Use line number as identifier
                let location = format!(
                    "{}:{}",
                    path.file_name().unwrap().to_string_lossy(),
                    line_num + 1
                );

                facts.loop_bounds.insert(
                    format!("loop_{}", line_num),
                    LoopBound {
                        location,
                        min_iterations: min,
                        max_iterations: max,
                    },
                );
            }

            // Check for entrypoint pragma
            if entrypoint_re.is_match(line) {
                // Function name might be on same line or next line
                // Try same line first
                if let Some(func_name) = Self::extract_function_name(line) {
                    facts.entry_point = Some(func_name);
                } else {
                    // Try next non-empty line
                    for next_line in &lines[line_num + 1..] {
                        let trimmed = next_line.trim();
                        if !trimmed.is_empty() && !trimmed.starts_with("//") {
                            if let Some(func_name) = Self::extract_function_name(trimmed) {
                                facts.entry_point = Some(func_name);
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(facts)
    }

    /// Extract function name from a function declaration line
    fn extract_function_name(line: &str) -> Option<String> {
        // Match patterns like: void function_name( ... )
        // Also handle: void _Pragma(...) function_name( ... )

        // First, remove _Pragma(...) if present
        let cleaned = if line.contains("_Pragma") {
            // Remove _Pragma( "..." ) part
            let pragma_re = Regex::new(r#"_Pragma\s*\([^)]+\)\s*"#).unwrap();
            pragma_re.replace(line, "").to_string()
        } else {
            line.to_string()
        };

        // Now extract function name
        let func_re = Regex::new(r"(\w+)\s*\(").unwrap();

        // Find all matches and take the last one (function name, not return type)
        let matches: Vec<_> = func_re.captures_iter(&cleaned).collect();

        if let Some(last_match) = matches.last() {
            let name = last_match.get(1).unwrap().as_str();
            // Skip return types
            if name != "void"
                && name != "int"
                && name != "long"
                && name != "char"
                && name != "float"
                && name != "double"
                && name != "unsigned"
                && name != "signed"
                && name != "const"
            {
                return Some(name.to_string());
            }
        }
        None
    }

    /// Get total number of loop bounds
    pub fn loop_count(&self) -> usize {
        self.loop_bounds.len()
    }

    /// Get maximum iterations across all loops
    pub fn max_total_iterations(&self) -> u32 {
        self.loop_bounds.values().map(|b| b.max_iterations).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_loopbound() {
        let source = r#"
void test() {
    _Pragma( "loopbound min 1 max 10" )
    for (int i = 0; i < 10; i++) {
        // loop body
    }
}
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();

        let facts = FlowFacts::parse_from_source(file.path()).unwrap();
        assert_eq!(facts.loop_count(), 1);

        let bound = facts.loop_bounds.values().next().unwrap();
        assert_eq!(bound.min_iterations, 1);
        assert_eq!(bound.max_iterations, 10);
    }

    #[test]
    fn test_parse_entrypoint() {
        let source = r#"
void _Pragma( "entrypoint" ) main_function( void )
{
    // function body
}
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();

        let facts = FlowFacts::parse_from_source(file.path()).unwrap();
        assert_eq!(facts.entry_point, Some("main_function".to_string()));
    }
}
