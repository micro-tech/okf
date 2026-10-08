use okf::config::load_server_config;

#[test]
fn test_load_server_config_success() {
    let cfg = load_server_config("config/server.yaml").expect("Failed to load server config");

    assert_eq!(cfg.server.host, "0.0.0.0");
    assert_eq!(cfg.server.port, 8080);
    assert_eq!(cfg.paths.bundles_dir, "./bundles");
    assert_eq!(cfg.paths.default_bundle, "shared");
}

#[test]
fn test_load_server_config_missing_file() {
    let result = load_server_config("config/does_not_exist.yaml");
    assert!(result.is_err());
}
