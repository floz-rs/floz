use floz::db::PoolOptions;

#[test]
fn test_pool_options_default() {
    let opts = PoolOptions::default();
    assert_eq!(opts.min_connections, 4);
    assert_eq!(opts.max_connections, 10);
    assert_eq!(opts.acquire_timeout_secs, 20);
    assert_eq!(opts.idle_timeout_secs, 300);
    assert_eq!(opts.max_lifetime_secs, 900);
}

// Notice: We do not test `floz::db::pool(0).await` directly here because it requires
// a live PostgreSQL connection out-of-the-box (sqlx strict checking).
