//! Proves, rather than asserts, decision 6: the held lock is a value with no public
//! constructor, and every function that writes takes one. Each failing case has a compiling
//! twin, so a fixture that fails to compile for an unrelated reason cannot pass unnoticed —
//! `trybuild` reports a fixture in the wrong list as a failure of this test either way.

#[test]
fn one_lock_acquisition_path_and_no_write_without_one() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/construct_lock_outside_module.rs");
    t.pass("tests/trybuild/construct_lock_via_acquire_ok.rs");
    t.compile_fail("tests/trybuild/write_without_lock.rs");
    t.pass("tests/trybuild/write_with_lock_ok.rs");
}
