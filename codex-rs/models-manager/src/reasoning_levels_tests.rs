use super::*;
use crate::manager::StaticModelsManager;
use crate::model_info::model_info_from_slug;
use codex_http_client::OutboundProxyPolicy;
use pretty_assertions::assert_eq;

const HTTP_CLIENT_FACTORY: HttpClientFactory =
    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault);

fn model(slug: &str, levels: &[(ReasoningEffort, &str)]) -> ModelInfo {
    let mut model = model_info_from_slug(slug);
    model.supported_reasoning_levels = levels
        .iter()
        .map(|(effort, description)| ReasoningEffortPreset {
            effort: effort.clone(),
            description: (*description).to_string(),
        })
        .collect();
    model
}

fn catalog() -> Vec<ModelInfo> {
    vec![
        model(
            "fast-model",
            &[
                (ReasoningEffort::Low, "Fast responses"),
                (ReasoningEffort::Max, "Maximum reasoning depth"),
            ],
        ),
        model(
            "frontier-model",
            &[
                (ReasoningEffort::Max, "Maximum reasoning depth"),
                (ReasoningEffort::Ultra, "Maximum reasoning with delegation"),
            ],
        ),
    ]
}

fn manager(overrides: &[(&str, &[ReasoningEffort])]) -> SharedModelsManager {
    let inner: SharedModelsManager = Arc::new(StaticModelsManager::new(
        /*auth_manager*/ None,
        ModelsResponse { models: catalog() },
    ));
    let overrides: ReasoningLevelOverrides = overrides
        .iter()
        .map(|(slug, efforts)| ((*slug).to_string(), efforts.to_vec()))
        .collect();
    with_reasoning_level_overrides(inner, &overrides)
}

fn levels(models: &[ModelInfo], slug: &str) -> Vec<ReasoningEffortPreset> {
    models
        .iter()
        .find(|model| model.slug == slug)
        .expect("model should be in the catalog")
        .supported_reasoning_levels
        .clone()
}

#[tokio::test]
async fn configured_level_is_added_with_the_catalog_description() {
    let manager = manager(&[("fast-model", &[ReasoningEffort::Ultra])]);

    let models = manager
        .raw_model_catalog(RefreshStrategy::Offline, HTTP_CLIENT_FACTORY)
        .await
        .models;

    assert_eq!(
        levels(&models, "fast-model").last(),
        Some(&ReasoningEffortPreset {
            effort: ReasoningEffort::Ultra,
            description: "Maximum reasoning with delegation".to_string(),
        })
    );
    assert_eq!(
        levels(&models, "frontier-model"),
        levels(&catalog(), "frontier-model")
    );
}

#[tokio::test]
async fn already_advertised_levels_are_left_alone() {
    let manager = manager(&[(
        "fast-model",
        &[ReasoningEffort::Max, ReasoningEffort::Ultra],
    )]);

    let models = manager.try_get_remote_models().expect("catalog");

    assert_eq!(
        levels(&models, "fast-model")
            .into_iter()
            .map(|preset| preset.effort)
            .collect::<Vec<_>>(),
        vec![
            ReasoningEffort::Low,
            ReasoningEffort::Max,
            ReasoningEffort::Ultra
        ]
    );
}

#[tokio::test]
async fn model_info_reports_the_added_level() {
    let manager = manager(&[("fast-model", &[ReasoningEffort::Ultra])]);

    let model_info = manager
        .get_model_info("fast-model", &ModelsManagerConfig::default())
        .await;

    assert!(
        model_info
            .supported_reasoning_levels
            .iter()
            .any(|preset| preset.effort == ReasoningEffort::Ultra),
        "levels: {:?}",
        model_info.supported_reasoning_levels
    );
}

#[tokio::test]
async fn unknown_slugs_and_empty_overrides_leave_the_catalog_unchanged() {
    let manager = manager(&[
        ("missing-model", &[ReasoningEffort::Ultra]),
        ("fast-model", &[]),
    ]);

    let models = manager.try_get_remote_models().expect("catalog");

    assert_eq!(
        levels(&models, "fast-model"),
        levels(&catalog(), "fast-model")
    );
}

#[test]
fn empty_overrides_return_the_inner_manager() {
    let inner: SharedModelsManager = Arc::new(StaticModelsManager::new(
        /*auth_manager*/ None,
        ModelsResponse { models: catalog() },
    ));

    let wrapped = with_reasoning_level_overrides(inner.clone(), &ReasoningLevelOverrides::new());

    assert!(Arc::ptr_eq(&inner, &wrapped));
}
