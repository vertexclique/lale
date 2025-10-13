use anyhow::Result;
use llvm_ir::Module;
use std::path::Path;

/// LLVM IR parser wrapper
pub struct IRParser;

impl IRParser {
    /// Parse LLVM IR from file (.ll or .bc)
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Module> {
        let path = path.as_ref();

        // Determine format from extension
        let module = if path.extension().and_then(|s| s.to_str()) == Some("bc") {
            // Parse bitcode
            Module::from_bc_path(path).map_err(|e| {
                anyhow::anyhow!("Failed to parse bitcode file {}: {}", path.display(), e)
            })?
        } else {
            // Parse textual IR
            Module::from_ir_path(path)
                .map_err(|e| anyhow::anyhow!("Failed to parse IR file {}: {}", path.display(), e))?
        };

        Ok(module)
    }

    /// Parse multiple IR files
    pub fn parse_files<P: AsRef<Path>>(paths: &[P]) -> Result<Vec<Module>> {
        paths.iter().map(|path| Self::parse_file(path)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample_ir() {
        // Test with sample data if available
        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let result = IRParser::parse_file(sample_path);
            assert!(result.is_ok(), "Failed to parse sample IR file");
        }
    }
}
