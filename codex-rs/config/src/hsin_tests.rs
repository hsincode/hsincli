use super::*;
use crate::config_toml::ConfigToml;
use pretty_assertions::assert_eq;

#[test]
fn hsin_config_parses_and_enforces_history_policy() {
    let config: ConfigToml = toml::from_str(
        r#"
        [hsin]
        display_name = "Hsin"
        mascot = "none"
        workspace = "rounded"
        [hsin.fork]
        default_turns = 2
        max_turns = 3
        allow_all = false
    "#,
    )
    .unwrap();
    config.hsin.validate().unwrap();
    let policy = config.hsin.fork;
    assert_eq!(
        policy.resolve(/*requested*/ None),
        policy.resolve(Some("2"))
    );
    assert_eq!(policy.resolve(Some("  ")), policy.resolve(Some("2")));
    assert_eq!(
        policy.resolve(Some(" NONE ")),
        Ok(ForkTurns::Mode(ForkMode::None))
    );
    assert!(policy.resolve(Some("3")).is_ok());
    for value in [
        "all",
        " ALL ",
        "4",
        "0",
        "-1",
        "invalid",
        "99999999999999999999999999",
    ] {
        assert!(policy.resolve(Some(value)).is_err(), "{value}");
    }
}

#[test]
fn hsin_default_and_full_history_opt_in() {
    let config: ConfigToml = toml::from_str("").unwrap();
    assert_eq!(
        config.hsin.fork.resolve(/*requested*/ None),
        Ok(ForkTurns::Recent(NonZeroUsize::MIN))
    );
    assert!(config.hsin.fork.resolve(Some("all")).is_err());
    let config: ConfigToml = toml::from_str(
        r#"
        [hsin.fork]
        allow_all = true
        default_turns = "all"
    "#,
    )
    .unwrap();
    assert_eq!(
        config.hsin.fork.resolve(/*requested*/ None),
        Ok(ForkTurns::Mode(ForkMode::All))
    );
    config.hsin.validate().unwrap();
}

#[test]
fn hsin_rejects_invalid_configuration_before_spawning() {
    for input in [
        "[hsin]\nsubagent_model_selection = 'invalid'",
        "[hsin.fork]\ndefault_turns = 0",
        "[hsin.fork]\nmax_turns = 0",
        "[hsin.fork]\nallow_al = true",
    ] {
        assert!(toml::from_str::<ConfigToml>(input).is_err(), "{input}");
    }
    for input in [
        "[hsin.fork]\ndefault_turns = 'all'",
        "[hsin.fork]\ndefault_turns = 3\nmax_turns = 2",
        "[hsin.fork]\nallow_all = true\ndefault_turns = 'all'\nmax_turns = 2",
    ] {
        let config: ConfigToml = toml::from_str(input).unwrap();
        assert!(config.hsin.validate().is_err(), "{input}");
    }
}

#[test]
fn a_numeric_limit_is_described_exactly_as_it_is_enforced() {
    let config: ConfigToml = toml::from_str(
        r#"
        [hsin.fork]
        default_turns = 1
        max_turns = 3
        allow_all = false
    "#,
    )
    .unwrap();
    let policy = config.hsin.fork;
    let description = policy.tool_description();
    // The description is the only channel for the policy, since the reserved
    // A reserved `collaboration.spawn_agent` schema cannot carry an enum. Every value it offers must
    // resolve, and every value it withholds must not.
    assert!(description.contains("from 1 to 3"), "{description}");
    assert!(!description.contains("all"), "{description}");
    for value in ["none", "1", "2", "3"] {
        assert!(policy.resolve(Some(value)).is_ok(), "{value}");
    }
    for value in ["all", "4"] {
        assert!(policy.resolve(Some(value)).is_err(), "{value}");
    }
}

#[test]
fn a_numeric_limit_excludes_full_history_even_when_allow_all_is_set() {
    let config: ConfigToml = toml::from_str(
        r#"
        [hsin.fork]
        allow_all = true
        max_turns = 2
    "#,
    )
    .unwrap();
    let policy = config.hsin.fork;
    // `check` rejects `all` whenever a limit exists, so the description must not offer it.
    assert!(policy.resolve(Some("all")).is_err());
    assert!(!policy.tool_description().contains("all"));
}

#[test]
fn without_a_limit_the_description_carries_the_policy() {
    // Recent-turn counts are unbounded here, so the description states the rule instead of
    // a range.
    let permissive: ConfigToml = toml::from_str("[hsin.fork]\nallow_all = true").unwrap();
    assert!(permissive.hsin.fork.tool_description().contains("all"));

    let restrictive: ConfigToml = toml::from_str("").unwrap();
    // Naming `all` only to forbid it invites the model to try it anyway.
    assert!(!restrictive.hsin.fork.tool_description().contains("all"));
}
