//! Covers SPC-10.
//!
//! Each failing case has a compiling twin, so a fixture that fails to compile for an unrelated
//! reason cannot pass unnoticed.

#[test]
fn one_lock_acquisition_path_and_no_write_without_one() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/construct_lock_outside_module.rs");
    t.pass("tests/trybuild/construct_lock_via_acquire_ok.rs");
    t.compile_fail("tests/trybuild/write_without_lock.rs");
    t.pass("tests/trybuild/write_with_lock_ok.rs");
}
