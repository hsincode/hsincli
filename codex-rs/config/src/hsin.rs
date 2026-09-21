//! Fork-local settings kept together to minimize upstream merge conflicts.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::num::NonZeroUsize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct HsinConfig {
    pub fork: ForkPolicy,
    /// V2 guidance for choosing child models; explicit preserves upstream behavior.
    pub subagent_model_selection: SubagentModelSelection,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubagentModelSelection {
    /// Change models only when requested by the user, AGENTS.md, or a skill.
    #[default]
    Explicit,
    /// Let the parent choose an available model to suit each delegated task.
    Auto,
}

impl HsinConfig {
    pub fn validate(&self) -> Result<(), String> {
        self.fork.check(self.fork.default_turns).map(|_| ())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ForkPolicy {
    /// History inherited when fork_turns is omitted or blank: "none", "all", or N.
    pub default_turns: ForkTurns,
    /// Permit full history. Disabled by default, including legacy fork_context=true.
    pub allow_all: bool,
    /// Optional hard limit on recent turns. Full history is also rejected when set.
    pub max_turns: Option<NonZeroUsize>,
}

impl Default for ForkPolicy {
    fn default() -> Self {
        Self {
            default_turns: ForkTurns::Recent(NonZeroUsize::MIN),
            allow_all: false,
            max_turns: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ForkTurns {
    Recent(NonZeroUsize),
    Mode(ForkMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ForkMode {
    None,
    All,
}

impl std::fmt::Display for ForkTurns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Recent(n) => write!(f, "{n}"),
            Self::Mode(ForkMode::None) => f.write_str("none"),
            Self::Mode(ForkMode::All) => f.write_str("all"),
        }
    }
}

impl ForkPolicy {
    pub fn resolve(&self, requested: Option<&str>) -> Result<ForkTurns, String> {
        let turns = match requested.map(str::trim).filter(|value| !value.is_empty()) {
            None => self.default_turns,
            Some(value) if value.eq_ignore_ascii_case("none") => ForkTurns::Mode(ForkMode::None),
            Some(value) if value.eq_ignore_ascii_case("all") => ForkTurns::Mode(ForkMode::All),
            Some(value) => ForkTurns::Recent(value.parse().map_err(|_| {
                "fork_turns must be `none`, `all`, or a positive integer string".to_string()
            })?),
        };
        self.check(turns)
    }

    fn check(&self, turns: ForkTurns) -> Result<ForkTurns, String> {
        // Enforce this at execution as well as in tool guidance: a stale schema or
        // a model explicitly requesting all must never silently copy full history.
        match (turns, self.max_turns) {
            (ForkTurns::Mode(ForkMode::All), max) if !self.allow_all || max.is_some() => {
                Err("Full-history forks are disabled by hsin.fork; use fork_turns=\"none\" or a permitted positive integer.".into())
            }
            (ForkTurns::Recent(n), Some(max)) if n > max => {
                Err(format!("fork_turns exceeds hsin.fork.max_turns={max}; use fewer turns or \"none\"."))
            }
            (ForkTurns::Recent(_) | ForkTurns::Mode(ForkMode::All | ForkMode::None), _) => Ok(turns),
        }
    }

    /// The `fork_turns` guidance handed to the model.
    ///
    /// The policy lives here rather than in the schema because users can configure the
    /// reserved `collaboration.spawn_agent` namespace: any structural departure from the
    /// schema it has configured — an `enum` of the permitted values included — fails the
    /// whole request with a 400. `check` is what actually enforces the policy; this text
    /// is how the model learns it before spending a round trip.
    pub fn tool_description(&self) -> String {
        if let Some(max) = self.max_turns {
            // `all` is rejected outright whenever a numeric limit exists, so it is left
            // unnamed here as well. See `check`.
            return format!(
                "Optional history to inherit. Defaults to `{}`. Use `none` for no parent \
                 history, or a number from 1 to {max} for that many recent turns.",
                self.default_turns
            );
        }
        let full = if self.allow_all {
            " `all` inherits the whole conversation."
        } else {
            // Naming `all` only to forbid it invites the model to try it anyway.
            ""
        };
        format!(
            "Optional history to inherit. Defaults to `{}`. Use `none` for no parent history, \
             or a positive integer string for that many recent turns.{full}",
            self.default_turns
        )
    }
}

#[cfg(test)]
#[path = "hsin_tests.rs"]
mod tests;
