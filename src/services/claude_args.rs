//! Global Claude CLI argument flags on [`Config::args`].

use crate::config::Config;

/// Sets whether `--dangerously-skip-permissions` is enabled in the global args.
pub fn set_dangerously_skip_permissions(config: &mut Config, enabled: bool) {
    config.args.dangerously_skip_permissions = enabled;
}

/// Toggles `--dangerously-skip-permissions` and returns the new enabled state.
///
/// Used by the interactive args menu (Part 4).
pub fn toggle_dangerously_skip_permissions(config: &mut Config) -> bool {
    config.args.dangerously_skip_permissions = !config.args.dangerously_skip_permissions;
    config.args.dangerously_skip_permissions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_dangerously_skip_permissions_updates_global_arg() {
        let mut config = Config::default();

        set_dangerously_skip_permissions(&mut config, true);
        assert!(config.args.dangerously_skip_permissions);

        set_dangerously_skip_permissions(&mut config, false);
        assert!(!config.args.dangerously_skip_permissions);
    }

    #[test]
    fn toggle_dangerously_skip_permissions_toggles_and_returns_new_value() {
        let mut config = Config::default();
        assert!(!config.args.dangerously_skip_permissions);

        assert!(toggle_dangerously_skip_permissions(&mut config));
        assert!(config.args.dangerously_skip_permissions);

        assert!(!toggle_dangerously_skip_permissions(&mut config));
        assert!(!config.args.dangerously_skip_permissions);
    }
}
