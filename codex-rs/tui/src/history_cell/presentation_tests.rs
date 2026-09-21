use super::*;
use crate::exec_cell::CommandOutput;
use crate::exec_cell::new_active_exec_command;
use crate::history_cell::AgentMessageCell;
use crate::history_cell::new_user_prompt;
use codex_app_server_protocol::CommandExecutionSource;
use pretty_assertions::assert_eq;
use ratatui::text::Line;
use std::time::Duration;

#[test]
fn compact_history_preserves_text_and_expanded_tool_output() {
    let user = new_user_prompt("Run checks".into(), Vec::new(), Vec::new(), Vec::new());
    let answer = AgentMessageCell::new(
        vec![
            "Results:".into(),
            "".into(),
            "• literal bullet".into(),
            "    code".into(),
        ],
        /*is_first_line*/ true,
    );
    let mut tool = new_active_exec_command(
        "call".into(),
        vec!["check".into()],
        Vec::new(),
        CommandExecutionSource::Agent,
        /*interaction_input*/ None,
        /*animations_enabled*/ false,
    );
    tool.complete_call(
        "call",
        CommandOutput::new(
            /*exit_code*/ 1,
            "one\ntwo\nthree\nfour\nfive\nError: failed".into(),
        ),
        Duration::from_secs(1),
    );
    let expanded = tool.transcript_lines(/*width*/ 60);
    let mut snapshots = Vec::new();
    for (compact_mode, large_bullets) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        let mode = HistoryRenderMode::from_tui(&Tui {
            compact_mode,
            large_bullets,
            ..Default::default()
        });
        let cells: [&dyn HistoryCell; 3] = [&user, &tool, &answer];
        let rendered = cells
            .into_iter()
            .flat_map(|cell| cell.display_lines_for_mode(/*width*/ 60, mode))
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        snapshots.push(format!(
            "compact={compact_mode}, large_bullets={large_bullets}\n{rendered}"
        ));
        assert_eq!(tool.transcript_lines(/*width*/ 60), expanded);
        assert_eq!(answer.display_lines_for_mode(/*width*/ 60, mode).len(), 4);
    }
    insta::assert_snapshot!(snapshots.join("\n---\n"));
}

#[test]
fn large_marker_keeps_hyperlink_columns_and_continuations() {
    use crate::terminal_hyperlinks::TerminalHyperlink;
    let link = TerminalHyperlink::web(0..4, "https://example.com".into());
    let cell = AgentMessageCell::new_hyperlink_lines(
        vec![HyperlinkLine {
            line: Line::from("link"),
            hyperlinks: vec![link],
        }],
        /*is_first_line*/ true,
    );
    let mode = HistoryRenderMode::from_tui(&Tui {
        large_bullets: true,
        ..Default::default()
    });
    let mut expected = cell.display_hyperlink_lines(/*width*/ 40);
    expected[0].line.spans[0].content = "● ".into();
    assert_eq!(
        cell.display_hyperlink_lines_for_mode(/*width*/ 40, mode),
        expected
    );
    let continuation = AgentMessageCell::new(vec![Line::from("")], /*is_first_line*/ false);
    let compact = HistoryRenderMode::from_tui(&Tui {
        compact_mode: true,
        ..Default::default()
    });
    assert_eq!(
        continuation.display_lines_for_mode(/*width*/ 40, compact),
        continuation.display_lines(/*width*/ 40)
    );
}

#[test]
fn raw_history_ignores_visual_preferences() {
    let cell = AgentMessageCell::new(vec![Line::from("• original")], /*is_first_line*/ true);
    let mode = HistoryRenderMode::from_tui(&Tui {
        raw_output_mode: true,
        compact_mode: true,
        large_bullets: true,
        ..Default::default()
    });
    assert_eq!(
        cell.display_lines_for_mode(/*width*/ 40, mode),
        cell.raw_lines()
    );
}
