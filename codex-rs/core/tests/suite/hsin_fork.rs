use super::*;
use codex_config::hsin::ForkTurns;
use core_test_support::responses::mount_sse_once;
use pretty_assertions::assert_eq;
use std::num::NonZeroUsize;
use test_case::test_case;

// Exercise the real tool dispatch and inspect the child's outgoing model input,
// so a policy that only changes schema text cannot satisfy these tests.
#[test_case(None, 1, false, true, false; "default recent turn")]
#[test_case(None, 2, false, true, true; "configured recent turns")]
#[test_case(Some("none"), 1, false, false, false; "explicit no history")]
#[test_case(Some("all"), 1, false, false, false; "reject full history")]
#[test_case(Some("4"), 1, false, false, false; "reject over limit")]
#[test_case(Some("all"), 1, true, true, true; "opt in to full history")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hsin_fork_policy_controls_child_input(
    requested: Option<&str>,
    default_turns: usize,
    allow_all: bool,
    keeps_latest: bool,
    keeps_older: bool,
) -> Result<()> {
    let server = start_mock_server().await;
    let test = test_codex()
        .with_config(move |config| {
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("enable V2");
            config.model = Some(V2_DEFAULT_MODEL.into());
            config.agent_default_subagent_model = Some(V2_REQUESTED_MODEL.into());
            config.hsin.fork.default_turns = ForkTurns::Recent(
                NonZeroUsize::new(default_turns).expect("positive test turn count"),
            );
            config.hsin.fork.allow_all = allow_all;
            config.hsin.fork.max_turns = if allow_all {
                None
            } else {
                NonZeroUsize::new(/*n*/ 3)
            };
        })
        .build_with_auto_env(&server)
        .await?;

    let _seed = mount_sse_once(&server, sse(vec![ev_completed("seed")])).await;
    test.submit_turn("HSIN_OLDER_HISTORY_MARKER").await?;

    let mut args = json!({"task_name": "hsin_worker", "message": "HSIN_CHILD_TASK"});
    if let Some(requested) = requested {
        args["fork_turns"] = json!(requested);
    }
    let spawn = mount_sse_once(
        &server,
        sse(vec![
            ev_function_call_with_namespace(
                SPAWN_CALL_ID,
                MULTI_AGENT_V2_NAMESPACE,
                "spawn_agent",
                &args.to_string(),
            ),
            ev_completed("spawn"),
        ]),
    )
    .await;
    let child = mount_sse_once_match(
        &server,
        |req: &wiremock::Request| {
            decoded_body(req)
                .and_then(|body| serde_json::from_slice::<Value>(&body).ok())
                .is_some_and(|body| body["model"] == V2_REQUESTED_MODEL)
        },
        sse(vec![
            ev_assistant_message("child", "done"),
            ev_completed("child"),
        ]),
    )
    .await;
    let parent = mount_sse_once(&server, sse(vec![ev_completed("parent")])).await;
    test.submit_turn("HSIN_LATEST_HISTORY_MARKER").await?;

    let rejected = !allow_all && matches!(requested, Some("all" | "4"));
    if rejected {
        // The recorder runs before the model matcher and also sees parent calls.
        assert!(
            child
                .requests()
                .iter()
                .all(|request| request.body_json()["model"] != V2_REQUESTED_MODEL)
        );
        assert!(
            parent
                .single_request()
                .function_call_output(SPAWN_CALL_ID)
                .to_string()
                .contains("hsin.fork")
        );
    } else {
        let request = wait_for_request_with_model(&child, V2_REQUESTED_MODEL).await?;
        assert!(request.body_contains_text("HSIN_CHILD_TASK"));
        assert_eq!(
            request.body_contains_text("HSIN_LATEST_HISTORY_MARKER"),
            keeps_latest
        );
        assert_eq!(
            request.body_contains_text("HSIN_OLDER_HISTORY_MARKER"),
            keeps_older
        );
    }
    // Code mode can embed tool schemas in context instead of the tools array.
    assert!(spawn.single_request().body_contains_text(if allow_all {
        "`all` is also allowed"
    } else {
        "`all` is disabled"
    }));
    Ok(())
}
