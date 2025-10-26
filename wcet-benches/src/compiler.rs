use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Benchmark compiler configured for ARM Cortex-M7 target
pub struct BenchmarkCompiler {
    clang_path: PathBuf,
    optimization_level: OptLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
}

impl BenchmarkCompiler {
    /// Create new compiler instance with default configuration
    pub fn new() -> Result<Self> {
        Ok(Self {
            clang_path: which::which("clang")
                .context("clang not found in PATH")?,
            optimization_level: OptLevel::O0,
        })
    }
    
    /// Set optimization level (default: O0)
    pub fn with_optimization(mut self, level: OptLevel) -> Self {
        self.optimization_level = level;
        self
    }
    
    /// Compile C source to LLVM IR for ARM Cortex-M7 target
    pub fn compile(
        &self,
        source: &Path,
        output: &Path,
    ) -> Result<()> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let mut cmd = Command::new(&self.clang_path);
        
        cmd.arg("-S")
           .arg("-emit-llvm")
           .arg(format!("-{}", self.opt_flag()));
        
        // ARM Cortex-M7 target configuration
        cmd.arg("--target=armv7em-none-eabi")
           .arg("-mcpu=cortex-m7")
           .arg("-mfloat-abi=hard")
           .arg("-mfpu=fpv5-d16");
        
        cmd.arg("-o").arg(output);
        cmd.arg(source);
        
        let output_result = cmd.output()
            .context("Failed to execute clang")?;
        
        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            anyhow::bail!(
                "Compilation failed for {}: {}",
                source.display(),
                stderr
            );
        }
        
        Ok(())
    }
    
    /// Compile multiple sources in batch
    pub fn compile_batch(
        &self,
        sources: &[PathBuf],
        output_dir: &Path,
    ) -> Result<Vec<CompilationResult>> {
        let mut results = Vec::new();
        
        for source in sources {
            let filename = source.file_stem()
                .context("Invalid source filename")?
                .to_string_lossy();
            let output = output_dir.join(format!("{}.ll", filename));
            
            let result = match self.compile(source, &output) {
                Ok(_) => CompilationResult {
                    source: source.clone(),
                    output: output.clone(),
                    success: true,
                    error: None,
                },
                Err(e) => CompilationResult {
                    source: source.clone(),
                    output,
                    success: false,
                    error: Some(e.to_string()),
                },
            };
            
            results.push(result);
        }
        
        Ok(results)
    }
    
    fn opt_flag(&self) -> &str {
        match self.optimization_level {
            OptLevel::O0 => "O0",
            OptLevel::O1 => "O1",
            OptLevel::O2 => "O2",
            OptLevel::O3 => "O3",
        }
    }
    
    /// Get target triple string
    pub fn target_triple() -> &'static str {
        "armv7em-none-eabi"
    }
    
    /// Get CPU name
    pub fn cpu_name() -> &'static str {
        "cortex-m7"
    }
}

#[derive(Debug)]
pub struct CompilationResult {
    pub source: PathBuf,
    pub output: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

impl Default for BenchmarkCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create default compiler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compiler_creation() {
        let compiler = BenchmarkCompiler::new();
        assert!(compiler.is_ok(), "Should find clang in PATH");
    }
    
    #[test]
    fn test_optimization_levels() {
        let compiler = BenchmarkCompiler::new().unwrap()
            .with_optimization(OptLevel::O0);
        assert_eq!(compiler.opt_flag(), "O0");
        
        let compiler = compiler.with_optimization(OptLevel::O2);
        assert_eq!(compiler.opt_flag(), "O2");
    }
    
    #[test]
    fn test_target_info() {
        assert_eq!(BenchmarkCompiler::target_triple(), "armv7em-none-eabi");
        assert_eq!(BenchmarkCompiler::cpu_name(), "cortex-m7");
    }
}
