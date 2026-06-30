mod common;

#[test]
fn crate_exports_config_module_when_initialized() {
    let config = cc_profile::config::Config::default();
    assert_eq!(config.version, 1);
    assert!(config.active_profile.is_none());
}
