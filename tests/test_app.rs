use floz::app::App;
use floz::config::Config;
use floz::server::ServerConfig;
use std::net::{IpAddr, Ipv4Addr};

#[test]
fn test_app_new() {
    // App::new() shouldn't panic without env vars since it evaluates them later
    let _app = App::new();
}

#[test]
fn test_server_config_defaults() {
    let config = ServerConfig::default();
    let addr = config.get_socket_addr();
    // Default is 127.0.0.1:3030
    assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(addr.port(), 3030);
}

#[test]
fn test_server_config_builder() {
    let config = ServerConfig::new()
        .with_default_host(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))
        .with_default_port(8080);

    let addr = config.get_socket_addr();
    assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
    assert_eq!(addr.port(), 8080);
}

#[test]
fn test_config_builder_overrides() {
    // We create a dummy Config manually since from_env() requires DATABASE_URL
    let custom_config = Config {
        database_url: "postgres://dummy".to_string(),
        host: "0.0.0.0".to_string(),
        port: "9000".to_string(),
        server_env: "TEST".to_string(),
        redis_url: None,
        jwt_secret: None,
        jwt_audience: None,
        jwt_issuer: None,
        tls_cert_path: None,
        tls_key_path: None,
        echo: false,
    };

    let _app = App::new()
        .config(custom_config)
        .server(ServerConfig::new().with_default_port(9000));

    // We can't easily assert on app internal state in integration tests because fields are private,
    // but we can ensure the builder methods exist and are chainable.
}
