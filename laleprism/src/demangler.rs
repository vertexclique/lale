use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemangledName {
    pub original: String,
    pub demangled: String,
    pub language: SymbolLanguage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolLanguage {
    Rust,
    Cpp,
    C,
    Unknown,
}

/// Demangle a symbol name (Rust or C++)
pub fn demangle_symbol(mangled: &str) -> DemangledName {
    // Try Rust demangling first
    if let Ok(demangled) = rustc_demangle::try_demangle(mangled) {
        return DemangledName {
            original: mangled.to_string(),
            demangled: format!("{:#}", demangled),
            language: SymbolLanguage::Rust,
        };
    }

    // Try C++ demangling
    if let Ok(sym) = cpp_demangle::Symbol::new(mangled) {
        return DemangledName {
            original: mangled.to_string(),
            demangled: format!("{:#}", sym),
            language: SymbolLanguage::Cpp,
        };
    }

    // Check if it's a C symbol (no mangling)
    if !mangled.starts_with('_') || mangled.len() < 3 {
        return DemangledName {
            original: mangled.to_string(),
            demangled: mangled.to_string(),
            language: SymbolLanguage::C,
        };
    }

    // Unknown or already demangled
    DemangledName {
        original: mangled.to_string(),
        demangled: mangled.to_string(),
        language: SymbolLanguage::Unknown,
    }
}

/// Demangle multiple symbols
pub fn demangle_batch(symbols: Vec<String>) -> Vec<DemangledName> {
    symbols.iter().map(|s| demangle_symbol(s)).collect()
}

/// Extract a human-readable function name from demangled output
pub fn extract_function_name(demangled: &str) -> String {
    // Remove generic parameters and return type
    let without_generics = demangled
        .split('<')
        .next()
        .unwrap_or(demangled);
    
    // Get the last component (function name)
    without_generics
        .split("::")
        .last()
        .unwrap_or(without_generics)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_demangling() {
        let mangled = "_ZN4core3ptr85drop_in_place$LT$std..rt..lang_start$LT$$LP$$RP$$GT$..$u7b$$u7b$closure$u7d$$u7d$$GT$17h1234567890abcdefE";
        let result = demangle_symbol(mangled);
        assert!(matches!(result.language, SymbolLanguage::Rust));
        assert_ne!(result.demangled, result.original);
    }

    #[test]
    fn test_cpp_demangling() {
        let mangled = "_ZN9wikipedia7article6formatEv";
        let result = demangle_symbol(mangled);
        assert!(matches!(result.language, SymbolLanguage::Cpp));
    }

    #[test]
    fn test_c_symbol() {
        let mangled = "printf";
        let result = demangle_symbol(mangled);
        assert!(matches!(result.language, SymbolLanguage::C));
        assert_eq!(result.demangled, "printf");
    }

    #[test]
    fn test_extract_function_name() {
        let demangled = "core::ptr::drop_in_place<std::rt::lang_start>";
        let name = extract_function_name(demangled);
        assert_eq!(name, "drop_in_place");
    }
}
