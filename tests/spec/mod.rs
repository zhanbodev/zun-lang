use std::fs;
use std::path::Path;

use zun::runtime::RuntimeMode;

use crate::support::{read_text, run_program_at_path};

pub fn assert_valid_spec(relative: &str) {
    let script_path = crate::support::fixture_path(relative);
    let expected_stdout = read_text(&format!("{relative}.stdout"));
    let outcome = run_program_at_path(&script_path, RuntimeMode::Test);
    assert!(
        outcome.error.is_none(),
        "valid spec {relative} should succeed"
    );
    assert_eq!(outcome.stdout, expected_stdout);
}

pub fn assert_invalid_spec(relative: &str) {
    let script_path = crate::support::fixture_path(relative);
    let expected_error = read_text(&format!("{relative}.error"));
    let outcome = run_program_at_path(&script_path, RuntimeMode::Test);
    let actual_error = outcome
        .error
        .unwrap_or_else(|| panic!("invalid spec {relative} should fail"));
    assert!(
        actual_error.contains(expected_error.trim()),
        "expected error snippet {:?}, got {:?}",
        expected_error.trim(),
        actual_error
    );
}

pub fn collect_specs(root: &Path, companion_extension: &str) -> Vec<String> {
    let mut specs = Vec::new();
    collect_specs_recursive(root, companion_extension, &mut specs);
    specs.sort();
    specs
}

fn collect_specs_recursive(root: &Path, companion_extension: &str, specs: &mut Vec<String>) {
    for entry in fs::read_dir(root).expect("spec dir should exist") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_specs_recursive(&path, companion_extension, specs);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("zun") {
            continue;
        }

        let companion = path.with_extension(format!("zun.{companion_extension}"));
        if !companion.exists() {
            continue;
        }

        let relative = path
            .strip_prefix(crate::support::fixture_path(""))
            .expect("path should be under tests/")
            .to_string_lossy()
            .to_string();
        specs.push(relative);
    }
}
