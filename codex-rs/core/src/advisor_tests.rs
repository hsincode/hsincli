use super::*;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use pretty_assertions::assert_eq;

fn user(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn call(name: &str, args: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: name.to_string(),
        namespace: None,
        arguments: args.to_string(),
        encrypted_function_args: None,
        call_id: "call_1".to_string(),
        internal_chat_message_metadata_passthrough: None,
    }
}

fn output(text: &str) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call_1".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload {
            body: FunctionCallOutputBody::Text(text.to_string()),
            success: Some(true),
        },
        internal_chat_message_metadata_passthrough: None,
    }
}

#[test]
fn transcript_carries_tool_calls_and_their_results() {
    let items = vec![
        user("add a flag"),
        call("shell", "pytest -q"),
        output("No module named pytest"),
    ];
    let rendered = render_transcript(&items, 100_000);
    assert!(rendered.contains("add a flag"));
    assert!(rendered.contains("tool call: shell"));
    // The failing result is the whole point: an agent's own summary tends to omit it.
    assert!(rendered.contains("No module named pytest"));
}

#[test]
fn transcript_keeps_the_task_statement_when_it_truncates() {
    let mut items = vec![user("THE ORIGINAL TASK")];
    for index in 0..200 {
        items.push(call("shell", &format!("command number {index}")));
        items.push(output(&"x".repeat(2_000)));
    }
    let rendered = render_transcript(&items, 20_000);
    assert!(rendered.contains("THE ORIGINAL TASK"));
    assert!(rendered.contains("earlier turns omitted"));
    // The tail is what the completion claim rests on, so the last command must survive.
    assert!(rendered.contains("command number 199"));
}

#[test]
fn reasoning_is_skipped_because_it_is_encrypted() {
    let items = vec![
        ResponseItem::Reasoning {
            id: None,
            summary: Vec::new(),
            content: None,
            encrypted_content: Some("gAAAAAB_opaque".to_string()),
            internal_chat_message_metadata_passthrough: None,
        },
        user("visible"),
    ];
    let rendered = render_transcript(&items, 100_000);
    assert!(!rendered.contains("gAAAAAB_opaque"));
    assert!(rendered.contains("visible"));
}

#[test]
fn a_turn_without_tool_calls_is_not_worth_reviewing() {
    assert!(!should_consult(&[user("what does this do?")], 1));
    assert!(should_consult(&[user("fix it"), call("shell", "ls")], 1));
    // Zero means "always", for anyone who wants every turn reviewed.
    assert!(should_consult(&[user("hello")], 0));
}

#[test]
fn only_a_bare_lgtm_counts_as_clear() {
    assert!(matches!(classify("LGTM"), AdvisorOutcome::Clear));
    assert!(matches!(classify("  lgtm \n"), AdvisorOutcome::Clear));
    assert!(matches!(classify(""), AdvisorOutcome::Clear));
    // A worded approval still costs one turn rather than being guessed at.
    assert!(matches!(
        classify("LGTM, though consider renaming the flag"),
        AdvisorOutcome::Guidance(_)
    ));
    match classify("The new tests never ran.") {
        AdvisorOutcome::Guidance(text) => assert_eq!(text, "The new tests never ran."),
        AdvisorOutcome::Clear => panic!("findings must reach the agent"),
    }
}

#[test]
fn long_tool_output_keeps_both_ends() {
    let body = format!("START{}END", "-".repeat(50_000));
    let rendered = render_transcript(&[output(&body)], 100_000);
    assert!(rendered.contains("START"));
    assert!(rendered.contains("END"));
    assert!(rendered.contains("characters omitted"));
}

#[test]
fn config_toml_resolves_the_documented_surface() {
    use crate::config::AdvisorConfig;
    use codex_config::config_toml::ConfigToml;

    let cfg: ConfigToml = toml::from_str(
        r#"
[advisor]
enabled = true
model = "gpt-5.6-sol"
reasoning_effort = "xhigh"
max_consultations_per_turn = 2
min_tool_calls = 3
max_transcript_chars = 120000
instructions = "be terse"
"#,
    )
    .expect("advisor table parses");
    let resolved = crate::config::resolve_advisor_config_for_test(&cfg);
    assert!(resolved.enabled);
    assert_eq!(resolved.model.as_deref(), Some("gpt-5.6-sol"));
    assert_eq!(resolved.max_consultations_per_turn, 2);
    assert_eq!(resolved.min_tool_calls, 3);
    assert_eq!(resolved.max_transcript_chars, 120_000);
    assert_eq!(resolved.instructions.as_deref(), Some("be terse"));

    // Omitting the table leaves the advisor off, so nothing is billed by default.
    let empty: ConfigToml = toml::from_str("").expect("empty config parses");
    assert_eq!(
        crate::config::resolve_advisor_config_for_test(&empty),
        AdvisorConfig::default()
    );

    // A zero consultation budget would otherwise mean "consult forever".
    let zero: ConfigToml =
        toml::from_str("[advisor]\nenabled = true\nmax_consultations_per_turn = 0\n")
            .expect("parses");
    assert_eq!(
        crate::config::resolve_advisor_config_for_test(&zero).max_consultations_per_turn,
        1
    );
}
