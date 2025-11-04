//! Test to reproduce LLVM crash with empty modules

use lale::InkwellAsyncDetector;

#[test]
fn test_empty_module_handling() {
    // Test with the problematic file
    let test_file = "../ral/pin_project_lite-413c347433af3cfc.ll";

    if !std::path::Path::new(test_file).exists() {
        eprintln!("Test file not found, skipping");
        return;
    }

    println!("Testing empty module file: {}", test_file);

    // This should not crash
    match InkwellAsyncDetector::detect_from_file(test_file) {
        Ok(funcs) => {
            println!("Success: Found {} functions", funcs.len());
            assert_eq!(funcs.len(), 0, "Empty module should have 0 functions");
        }
        Err(e) => {
            println!("Error (expected for empty module): {}", e);
        }
    }
}

#[test]
fn test_minimal_empty_module() {
    let empty_ir = r#"; ModuleID = 'test'
source_filename = "test"
target datalayout = "e-m:e-p:32:32-Fi8-i64:64-v128:64:128-a:0:32-n32-S64"
target triple = "armv7r-unknown-none-eabihf"

!llvm.ident = !{!0}
!0 = !{!"test"}
"#;

    println!("Testing minimal empty module");

    match InkwellAsyncDetector::detect_from_ir_text(empty_ir) {
        Ok(funcs) => {
            println!("Success: Found {} functions", funcs.len());
            assert_eq!(funcs.len(), 0);
        }
        Err(e) => {
            println!("Error: {}", e);
            // Empty modules might fail to parse, that's ok
        }
    }
}
