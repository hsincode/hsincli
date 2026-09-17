use crate::config_toml::AgentsToml;
use crate::config_toml::ConfigToml;
use pretty_assertions::assert_eq;

#[test]
fn parses_subagent_service_tier_wire_values() {
    for tier in ["default", "priority", "flex"] {
        let config: ConfigToml = toml::from_str(&format!(
            "[agents]\ndefault_subagent_service_tier = {tier:?}"
        ))
        .unwrap();
        assert_eq!(
            config.agents,
            Some(AgentsToml {
                default_subagent_service_tier: Some(tier.to_string()),
                ..Default::default()
            })
        );
    }
    let config: ConfigToml = toml::from_str("[agents]").unwrap();
    assert_eq!(config.agents, Some(AgentsToml::default()));
}

#[test]
fn rejects_invalid_subagent_service_tiers() {
    for tier in ["fast", "prioritty", "", "Priority", " priority "] {
        let error = toml::from_str::<ConfigToml>(&format!(
            "[agents]\ndefault_subagent_service_tier = {tier:?}"
        ))
        .unwrap_err()
        .to_string();
        assert!(error.contains("default_subagent_service_tier"), "{error}");
        assert!(
            error.contains("expected one of `default`, `priority`, `flex`"),
            "{error}"
        );
    }
}
