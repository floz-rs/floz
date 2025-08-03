#![cfg(feature = "logger")]

use floz::logger::init_tracing;

#[test]
fn test_init_tracing() {
    // Should initialize tracing and not panic.
    // Multiple calls should be safe due to Once struct.
    init_tracing();
    init_tracing();
}
