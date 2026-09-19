//! Consult a stronger model at the point where a turn would otherwise finish.
//!
//! The multi-agent tools already let a model delegate, but delegation is chosen by the
//! model, and a run that ends convinced it is finished is exactly the one that never asks
//! for a second opinion. Measured on a DeepSWE run, every failure of the stronger arm was
//! a turn that declared success on a verification it had not actually performed — one
//! reported "295 passed" for a suite that never covered the new behaviour, another said
//! tests could not run while its own tool output carried `No module named pytest`.
//!
//! So this fires on a rule rather than on the model's judgement, and it reads the
//! conversation itself rather than asking the agent to describe it: a summary written by
//! the agent inherits the same mistaken belief.

use std::sync::Arc;

use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::plaintext_agent_message_content;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::user_input::UserInput;
use tokio_util::sync::CancellationToken;

use crate::codex_delegate::run_codex_thread_one_shot;
use crate::config::Constrained;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use codex_features::Feature;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::models::BaseInstructionsProvenance;

/// Tool output longer than this is kept head-and-tail: a command and its verdict usually
/// sit at opposite ends, and the middle is rarely what decides the question.
const MAX_TOOL_OUTPUT_CHARS: usize = 4_000;

pub(crate) const DEFAULT_ADVISOR_INSTRUCTIONS: &str = r#"You are the advisor. Another agent has been working on a task and is about to report it
complete. You are given the conversation as it actually happened, including every tool
call and its result, and your answer goes back to that agent before it finishes.

Judge whether the work is actually done, using the transcript rather than the agent's
account of it. The two often disagree, and when they do the transcript is what counts.
Check in particular:

- Was the behaviour the task asked for actually exercised, or only code that already
  worked? A large passing count means nothing if it never covered the new behaviour.
- Did any command fail, or report a missing tool or dependency, and then get treated as
  though it had succeeded?
- Does anything in the task statement remain unaddressed?

Reply with `LGTM` on its own line when you find nothing worth acting on. Otherwise state
what is wrong and what to do about it, most important first, specific enough to act on
without re-deriving your reasoning. Do not restate work that is already correct."#;

const ADVISOR_REQUEST_PREAMBLE: &str = r#"The agent below is about to report this task complete. Review the transcript and answer
as instructed."#;

/// How a consultation ended, for the caller's logging and its loop bound.
pub(crate) enum AdvisorOutcome {
    /// The advisor found something; the text goes back to the agent.
    Guidance(String),
    /// The advisor was satisfied, or produced nothing usable.
    Clear,
}

/// True when the turn did enough to be worth reviewing.
///
/// A turn that only answered a question has nothing to verify, and consulting on it would
/// bill a more expensive model for no possible finding.
pub(crate) fn should_consult(items: &[ResponseItem], min_tool_calls: usize) -> bool {
    if min_tool_calls == 0 {
        return true;
    }
    let calls = items
        .iter()
        .filter(|item| {
            matches!(
                item,
                ResponseItem::FunctionCall { .. }
                    | ResponseItem::CustomToolCall { .. }
                    | ResponseItem::LocalShellCall { .. }
            )
        })
        .count();
    calls >= min_tool_calls
}

fn clip(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let head = limit * 2 / 3;
    let tail = limit / 3;
    let dropped = chars.len() - head - tail;
    let head_text: String = chars[..head].iter().collect();
    let tail_text: String = chars[chars.len() - tail..].iter().collect();
    format!("{head_text}\n...[{dropped} characters omitted]...\n{tail_text}")
}

fn content_text(content: &[ContentItem]) -> String {
    content
        .iter()
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Renders the conversation for the advisor, newest-first budgeting.
///
/// Reasoning items are skipped: they are encrypted, so there is nothing to render, and the
/// tool calls around them carry what actually happened anyway.
pub(crate) fn render_transcript(items: &[ResponseItem], budget: usize) -> String {
    let mut blocks: Vec<String> = Vec::new();
    for item in items {
        let block = match item {
            ResponseItem::Message { role, content, .. } => {
                let text = content_text(content);
                if text.trim().is_empty() {
                    continue;
                }
                format!("## {role}\n{}", clip(text.trim(), MAX_TOOL_OUTPUT_CHARS))
            }
            ResponseItem::AgentMessage { content, .. } => {
                match plaintext_agent_message_content(content) {
                    Some(text) if !text.trim().is_empty() => {
                        format!("## assistant\n{}", clip(text.trim(), MAX_TOOL_OUTPUT_CHARS))
                    }
                    _ => continue,
                }
            }
            ResponseItem::FunctionCall {
                name, arguments, ..
            } => format!(
                "## tool call: {name}\n{}",
                clip(arguments, MAX_TOOL_OUTPUT_CHARS)
            ),
            ResponseItem::CustomToolCall { name, input, .. } => format!(
                "## tool call: {name}\n{}",
                clip(input, MAX_TOOL_OUTPUT_CHARS)
            ),
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => {
                let text = serde_json::to_string(&output.body).unwrap_or_default();
                format!("### result\n{}", clip(&text, MAX_TOOL_OUTPUT_CHARS))
            }
            _ => continue,
        };
        blocks.push(block);
    }

    // Fill from the end: the recent turns are what the completion claim rests on. The
    // first block is kept regardless, since it is the task statement everything else
    // refers to.
    let mut kept: Vec<&String> = Vec::new();
    let mut used = 0usize;
    for block in blocks.iter().rev() {
        if used + block.len() > budget {
            break;
        }
        used += block.len();
        kept.push(block);
    }
    kept.reverse();

    let mut rendered = String::new();
    if let Some(first) = blocks.first()
        && !kept.first().is_some_and(|kept| std::ptr::eq(*kept, first))
    {
        rendered.push_str(&clip(first, 20_000));
        rendered.push_str("\n\n[...earlier turns omitted...]\n\n");
    }
    rendered.push_str(
        &kept
            .into_iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n\n"),
    );
    rendered
}

/// Runs one consultation and returns the advisor's answer.
///
/// Failures are reported as `Clear` rather than propagated: an advisor that cannot be
/// reached should not fail the turn it was meant to improve.
pub(crate) async fn consult(
    session: Arc<Session>,
    turn: Arc<TurnContext>,
    cancellation_token: CancellationToken,
) -> AdvisorOutcome {
    let advisor = &turn.config.advisor;
    let snapshot = session.clone_history().await;
    let items: Vec<ResponseItem> = snapshot.raw_items().cloned().collect();

    if !should_consult(&items, advisor.min_tool_calls) {
        return AdvisorOutcome::Clear;
    }
    let transcript = render_transcript(&items, advisor.max_transcript_chars);
    if transcript.trim().is_empty() {
        return AdvisorOutcome::Clear;
    }

    let mut config = turn.config.as_ref().clone();
    config.model = Some(
        advisor
            .model
            .clone()
            .unwrap_or_else(|| turn.model_info().slug.clone()),
    );
    config.model_reasoning_effort = advisor.reasoning_effort.clone();
    config.base_instructions = Some(
        advisor
            .instructions
            .clone()
            .unwrap_or_else(|| DEFAULT_ADVISOR_INSTRUCTIONS.to_string()),
    );
    config.base_instructions_provenance = Some(BaseInstructionsProvenance::Custom);
    config.permissions.approval_policy = Constrained::allow_only(AskForApproval::Never);
    // The advisor answers from the transcript in front of it. Letting it delegate or
    // search would turn a bounded second opinion into another agent loop.
    let _ = config.features.disable(Feature::Collab);
    let _ = config.features.disable(Feature::MultiAgentV2);
    let _ = config.web_search_mode.set(WebSearchMode::Disabled);
    // An advisor that consulted an advisor would not terminate.
    config.advisor.enabled = false;

    let input = vec![UserInput::Text {
        text: format!("{ADVISOR_REQUEST_PREAMBLE}\n\n# Transcript\n\n{transcript}"),
        text_elements: Vec::new(),
    }];

    let delegate = run_codex_thread_one_shot(
        config,
        Arc::clone(&session.services.auth_manager),
        Arc::clone(&session.services.models_manager),
        input,
        Arc::clone(&session),
        Arc::clone(&turn),
        cancellation_token,
        SubAgentSource::Review,
        /*final_output_json_schema*/ None,
        /*initial_history*/ None,
    )
    .await;

    let receiver = match delegate {
        Ok((_session, io)) => io.rx_event,
        Err(err) => {
            tracing::warn!("advisor consultation could not start: {err}");
            return AdvisorOutcome::Clear;
        }
    };
    collect_answer(receiver).await
}

async fn collect_answer(receiver: async_channel::Receiver<Event>) -> AdvisorOutcome {
    while let Ok(event) = receiver.recv().await {
        match event.msg {
            EventMsg::TurnComplete(complete) => {
                let answer = complete.last_agent_message.unwrap_or_default();
                return classify(&answer);
            }
            EventMsg::TurnAborted(_) => return AdvisorOutcome::Clear,
            _ => {}
        }
    }
    AdvisorOutcome::Clear
}

/// `LGTM` on its own line is the agreed way to say "nothing to do"; anything else is
/// treated as guidance so a differently worded approval still costs only one extra turn.
fn classify(answer: &str) -> AdvisorOutcome {
    let trimmed = answer.trim();
    if trimmed.is_empty() {
        return AdvisorOutcome::Clear;
    }
    let is_clear = trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
        == 1
        && trimmed.eq_ignore_ascii_case("LGTM");
    if is_clear {
        AdvisorOutcome::Clear
    } else {
        AdvisorOutcome::Guidance(trimmed.to_string())
    }
}

/// Wraps the advisor's answer as the message handed back to the agent.
pub(crate) fn guidance_prompt(guidance: &str) -> String {
    format!(
        "An advisor reviewed this conversation before you finish. Address its findings, \
         then report. Where you disagree with a specific claim, say so and why rather \
         than following it anyway.\n\n{guidance}"
    )
}

#[cfg(test)]
#[path = "advisor_tests.rs"]
mod tests;
