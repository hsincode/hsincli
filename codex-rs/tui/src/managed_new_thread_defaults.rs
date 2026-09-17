use crate::legacy_core::config::Config;
use crate::legacy_core::config::ConfigOverrides;
use codex_app_server_protocol::NewThreadModelDefaults;
use codex_protocol::config_types::ServiceTier;
use toml::Value as TomlValue;

pub(crate) fn apply_managed_new_thread_defaults(
    config: &mut Config,
    defaults: Option<&NewThreadModelDefaults>,
    cli_kv_overrides: &[(String, TomlValue)],
    harness_overrides: &ConfigOverrides,
) {
    let Some(defaults) = defaults else {
        return;
    };
    // Managed values are defaults rather than enforcement. Preserve saved preferences and explicit
    // launch choices from dedicated flags such as `-m` (`harness_overrides`) and generic
    // `-c key=value` settings (`cli_kv_overrides`), then fill only fields with no user selection.
    // Model and reasoning effort are a compatibility-sensitive pair, so a configured or explicit
    // value for either opts out of both managed values. Service tier remains independent and is
    // resolved against the selected model before the thread starts.
    let has_cli_override = |key: &str| cli_kv_overrides.iter().any(|(path, _value)| path == key);
    let has_user_model_settings = config.model.is_some()
        || config.model_reasoning_effort.is_some()
        || harness_overrides.model.is_some()
        || has_cli_override("model")
        || has_cli_override("model_reasoning_effort");

    if !has_user_model_settings && let Some(model) = defaults.model.as_ref() {
        config.model = Some(model.clone());
    }
    if !has_user_model_settings
        && let Some(reasoning_effort) = defaults.model_reasoning_effort.as_ref()
    {
        config.model_reasoning_effort = Some(reasoning_effort.clone());
    }
    if harness_overrides.service_tier.is_none()
        && !has_cli_override("service_tier")
        && let Some(service_tier) = defaults.service_tier.as_ref()
    {
        config.service_tier = Some(
            ServiceTier::from_request_value(service_tier)
                .map(|tier| tier.request_value().to_string())
                .unwrap_or_else(|| service_tier.clone()),
        );
    }
}

#[cfg(test)]
#[path = "managed_new_thread_defaults_tests.rs"]
mod tests;
