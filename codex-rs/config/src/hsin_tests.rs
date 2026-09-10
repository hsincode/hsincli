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
