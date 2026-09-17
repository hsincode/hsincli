use schemars::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;

// Config values are wire IDs; display labels such as "fast" must not silently
// fall back to standard routing when checked against the model catalog.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SubagentServiceTier {
    Default,
    Priority,
    Flex,
}

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<SubagentServiceTier>::deserialize(deserializer).map(|tier| {
        tier.map(|tier| match tier {
            SubagentServiceTier::Default => "default".to_string(),
            SubagentServiceTier::Priority => "priority".to_string(),
            SubagentServiceTier::Flex => "flex".to_string(),
        })
    })
}

#[cfg(test)]
#[path = "subagent_service_tier_tests.rs"]
mod tests;
