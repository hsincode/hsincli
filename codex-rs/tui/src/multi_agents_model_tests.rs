use super::*;
use crate::history_cell::HistoryCell;
use pretty_assertions::assert_eq;

#[test]
fn spawn_model_without_effort_prefers_completed_model() {
    let item = ThreadItem::CollabAgentToolCall {
        id: "spawn".into(),
        tool: CollabAgentTool::SpawnAgent,
        status: CollabAgentToolCallStatus::Completed,
        sender_thread_id: "00000000-0000-0000-0000-000000000001".into(),
        receiver_thread_ids: vec!["00000000-0000-0000-0000-000000000002".into()],
        prompt: None,
        model: Some("child-model".into()),
        reasoning_effort: None,
        agents_states: Default::default(),
    };
    let requested = SpawnRequestSummary {
        model: "requested-model".into(),
        reasoning_effort: Some(ReasoningEffortConfig::High),
    };
    assert_eq!(
        spawn_request_summary(&item),
        Some(SpawnRequestSummary {
            model: "child-model".into(),
            reasoning_effort: None,
        })
    );
    let cell = tool_call_history_cell(&item, Some(&requested), |_| AgentMetadata {
        agent_nickname: Some("review".into()),
        agent_role: Some("worker".into()),
        ..Default::default()
    })
    .expect("completed spawn renders");
    insta::assert_snapshot!(
        cell.display_lines(/*width*/ 100)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn v2_started_subagent_shows_child_and_parent_models() {
    let item = ThreadItem::SubAgentActivity {
        id: "started".into(),
        kind: SubAgentActivityKind::Started,
        agent_thread_id: "00000000-0000-0000-0000-000000000002".into(),
        agent_path: "/root/review".into(),
    };
    let mut snapshots = Vec::new();
    for model in [None, Some("parent-model"), Some("child-model")] {
        let cell = sub_agent_activity_history_cell(&item, model, "parent-model").unwrap();
        snapshots.push(
            cell.display_lines(/*width*/ 100)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    insta::assert_snapshot!(snapshots.join("\n"));
}
