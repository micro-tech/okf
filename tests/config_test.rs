use okf::config::load_server_config;

#[test]
fn test_load_server_config_success() {
    let cfg = load_server_config("config/server.yaml").expect("Failed to load server config");

    assert_eq!(cfg.server.host, "127.0.0.1");
    assert_eq!(cfg.server.port, 8080);
    assert_eq!(cfg.paths.bundles_dir, "./bundles");
    assert_eq!(cfg.paths.default_bundle, "shared");
}

#[test]
fn test_load_server_config_missing_file() {
    let result = load_server_config("config/does_not_exist.yaml");
    assert!(result.is_err());
}

#[test]
fn test_validate_config_accepts_good_default_bundle() {
    use okf::config::validate_config;
    use okf::model::{AuthSection, PathsSection, ServerConfig, ServerSection};

    // GIVEN a config whose default_bundle exists on disk:
    let cfg = ServerConfig {
        server: ServerSection { host: "127.0.0.1".to_string(), port: 8080 },
        paths: PathsSection { bundles_dir: "bundles".to_string(), default_bundle: "helix".to_string() },
        auth: AuthSection { token: None },
    };

    // THEN validation passes:
    assert!(validate_config(&cfg).is_ok());
}

#[test]
fn test_validate_config_rejects_missing_default_bundle() {
    use okf::config::validate_config;
    use okf::model::{AuthSection, PathsSection, ServerConfig, ServerSection};

    // GIVEN a config whose default_bundle names nothing on disk:
    let cfg = ServerConfig {
        server: ServerSection { host: "127.0.0.1".to_string(), port: 8080 },
        paths: PathsSection { bundles_dir: "bundles".to_string(), default_bundle: "nope".to_string() },
        auth: AuthSection { token: None },
    };

    // THEN validation fails fast at startup instead of surprising later:
    assert!(validate_config(&cfg).is_err());
}

#[test]
fn test_auth_section_defaults_to_absent() {
    use okf::config::load_server_config;

    // GIVEN the shipped config (no token set):
    let cfg = load_server_config("config/server.yaml").expect("config should load");

    // THEN auth is absent → the server runs fail-open:
    assert!(cfg.auth.token.is_none());
}
