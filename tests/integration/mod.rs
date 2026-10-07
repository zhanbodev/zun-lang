use zun::interpreter::ZunValue;
use zun::runtime::RuntimeMode;

use crate::support::{run_fixture, run_route_fixture};

pub fn assert_fixture_stdout(relative: &str, expected: &str) {
    let outcome = run_fixture(relative, RuntimeMode::Test);
    assert!(outcome.error.is_none(), "fixture {relative} should succeed");
    assert_eq!(outcome.stdout, expected);
}

pub fn assert_route_string(relative: &str, method: &str, path: &str, expected: &str) {
    let outcome = run_route_fixture(relative, method, path, None);
    assert!(outcome.error.is_none(), "route should succeed");
    assert!(outcome.should_continue, "handler should complete normally");
    assert_eq!(outcome.result, Some(ZunValue::Str(expected.to_string())));
}
