//! Config-driven widening of the reasoning levels a model advertises.
//!
//! The catalog decides which levels each model offers, and under ChatGPT auth that catalog
//! comes from the server and replaces the bundled copy wholesale. Editing a local file
//! therefore cannot add a level. This wrapper sits in front of whichever manager the provider
//! built and adds the configured levels to every catalog snapshot it hands out, so the picker,
//! the effort popup, and the per-model checks in core all see the same list.
//!
//! Levels are only ever added, never removed, and a level the catalog already advertises is
//! left untouched. Descriptions are borrowed from another model advertising the same level so
//! the wording follows the catalog instead of drifting from it.

use std::collections::BTreeMap;
use std::sync::Arc;

use codex_http_client::HttpClientFactory;
use codex_login::AuthManager;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use tokio::sync::TryLockError;
use tracing::warn;

use crate::ModelsManagerConfig;
use crate::manager::ModelsManager;
use crate::manager::ModelsManagerFuture;
use crate::manager::RefreshStrategy;
use crate::manager::SharedModelsManager;

/// Extra reasoning levels to advertise, keyed by model slug.
pub type ReasoningLevelOverrides = BTreeMap<String, Vec<ReasoningEffort>>;

/// Wrap `inner` so configured models advertise extra reasoning levels.
///
/// Returns `inner` unchanged when nothing is configured, keeping the default path free of the
/// extra indirection.
pub fn with_reasoning_level_overrides(
    inner: SharedModelsManager,
    overrides: &ReasoningLevelOverrides,
) -> SharedModelsManager {
    let overrides: ReasoningLevelOverrides = overrides
        .iter()
        .filter(|(_, efforts)| !efforts.is_empty())
        .map(|(slug, efforts)| (slug.clone(), efforts.clone()))
        .collect();
    if overrides.is_empty() {
        return inner;
    }
    Arc::new(ReasoningLevelOverrideManager { inner, overrides })
}

#[derive(Debug)]
struct ReasoningLevelOverrideManager {
    inner: SharedModelsManager,
    overrides: ReasoningLevelOverrides,
}

impl ReasoningLevelOverrideManager {
    fn apply_to_catalog(&self, mut models: Vec<ModelInfo>) -> Vec<ModelInfo> {
        for (slug, efforts) in &self.overrides {
            let Some(index) = models.iter().position(|model| &model.slug == slug) else {
                warn!("model_reasoning_levels: no model named {slug} in the catalog");
                continue;
            };
            let added = added_levels(&models, &models[index].supported_reasoning_levels, efforts);
            models[index].supported_reasoning_levels.extend(added);
        }
        models
    }

    fn apply_to_model(&self, mut model: ModelInfo, catalog: &[ModelInfo]) -> ModelInfo {
        let Some(efforts) = self.overrides.get(&model.slug) else {
            return model;
        };
        let added = added_levels(catalog, &model.supported_reasoning_levels, efforts);
        model.supported_reasoning_levels.extend(added);
        model
    }
}

/// Presets for the configured levels that `supported` does not already advertise.
fn added_levels(
    catalog: &[ModelInfo],
    supported: &[ReasoningEffortPreset],
    efforts: &[ReasoningEffort],
) -> Vec<ReasoningEffortPreset> {
    let mut added: Vec<ReasoningEffortPreset> = Vec::new();
    for effort in efforts {
        let already_present = supported.iter().any(|preset| &preset.effort == effort)
            || added.iter().any(|preset| &preset.effort == effort);
        if already_present {
            continue;
        }
        added.push(ReasoningEffortPreset {
            effort: effort.clone(),
            description: catalog_description(catalog, effort),
        });
    }
    added
}

/// Description another model uses for `effort`, or an empty string when none advertises it.
fn catalog_description(catalog: &[ModelInfo], effort: &ReasoningEffort) -> String {
    catalog
        .iter()
        .flat_map(|model| model.supported_reasoning_levels.iter())
        .find(|preset| &preset.effort == effort && !preset.description.is_empty())
        .map(|preset| preset.description.clone())
        .unwrap_or_default()
}

impl ModelsManager for ReasoningLevelOverrideManager {
    // The trait default is a silent no-op, which would drop the startup discovery policy
    // whenever this wrapper is in front of the real manager.
    fn set_api_key_model_discovery_enabled(&self, enabled: bool) {
        self.inner.set_api_key_model_discovery_enabled(enabled);
    }

    fn raw_model_catalog(
        &self,
        refresh_strategy: RefreshStrategy,
        http_client_factory: HttpClientFactory,
    ) -> ModelsManagerFuture<'_, ModelsResponse> {
        Box::pin(async move {
            let catalog = self
                .inner
                .raw_model_catalog(refresh_strategy, http_client_factory)
                .await;
            ModelsResponse {
                models: self.apply_to_catalog(catalog.models),
            }
        })
    }

    fn get_remote_models(&self) -> ModelsManagerFuture<'_, Vec<ModelInfo>> {
        Box::pin(async move { self.apply_to_catalog(self.inner.get_remote_models().await) })
    }

    fn try_get_remote_models(&self) -> Result<Vec<ModelInfo>, TryLockError> {
        Ok(self.apply_to_catalog(self.inner.try_get_remote_models()?))
    }

    fn auth_manager(&self) -> Option<&AuthManager> {
        self.inner.auth_manager()
    }

    fn list_collaboration_modes(&self) -> Vec<CollaborationModeMask> {
        self.inner.list_collaboration_modes()
    }

    fn get_default_model<'a>(
        &'a self,
        model: &'a Option<String>,
        allow_provider_model_fallback: bool,
        refresh_strategy: RefreshStrategy,
        http_client_factory: HttpClientFactory,
    ) -> ModelsManagerFuture<'a, String> {
        // Effort levels do not affect which slug is selected, so the inner policy is preserved.
        self.inner.get_default_model(
            model,
            allow_provider_model_fallback,
            refresh_strategy,
            http_client_factory,
        )
    }

    fn get_model_info<'a>(
        &'a self,
        model: &'a str,
        config: &'a ModelsManagerConfig,
    ) -> ModelsManagerFuture<'a, ModelInfo> {
        Box::pin(async move {
            let catalog = self.inner.get_remote_models().await;
            let model_info = self.inner.get_model_info(model, config).await;
            self.apply_to_model(model_info, &catalog)
        })
    }

    fn refresh_if_new_etag(
        &self,
        etag: String,
        http_client_factory: HttpClientFactory,
    ) -> ModelsManagerFuture<'_, ()> {
        self.inner.refresh_if_new_etag(etag, http_client_factory)
    }
}

#[cfg(test)]
#[path = "reasoning_levels_tests.rs"]
mod tests;
