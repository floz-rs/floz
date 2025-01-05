// Root files testing (e.g. router)
// Note: Some root files like config and server were tested in test_app.rs

#[test]
fn test_prelude_reexports() {
    // Ensure prelude items don't panic on access/compilation
    let _ = floz::ntex::web::HttpResponse::Ok().finish();
}
