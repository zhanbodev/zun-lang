mod spec;
mod support;

#[test]
fn valid_spec_samples_pass() {
    for relative in spec::collect_specs(&support::fixture_path("spec/valid"), "stdout") {
        spec::assert_valid_spec(&relative);
    }
}

#[test]
fn invalid_spec_samples_fail_with_expected_error() {
    for relative in spec::collect_specs(&support::fixture_path("spec/invalid"), "error") {
        spec::assert_invalid_spec(&relative);
    }
}
