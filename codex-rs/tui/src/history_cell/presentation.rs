//! Display preferences shared by live history, replay, and terminal resize reflow.

use super::HistoryCell;
use super::HistoryRenderMode;
use super::McpToolCallCell;
use super::UserHistoryCell;
use crate::exec_cell::ExecCell;
use crate::terminal_hyperlinks::HyperlinkLine;
use codex_config::types::Tui;
use ratatui::style::Stylize;
use std::any::TypeId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HistoryPresentation {
    pub(crate) compact: bool,
    pub(crate) large_bullets: bool,
}

impl HistoryRenderMode {
    pub(crate) fn from_tui(tui: &Tui) -> Self {
        if tui.raw_output_mode {
            Self::Raw
        } else if tui.compact_mode || tui.large_bullets {
            Self::Styled(HistoryPresentation {
                compact: tui.compact_mode,
                large_bullets: tui.large_bullets,
            })
        } else {
            Self::Rich
        }
    }

    pub(crate) fn is_compact(self) -> bool {
        matches!(
            self,
            Self::Styled(HistoryPresentation { compact: true, .. })
        )
    }
}

impl HistoryPresentation {
    pub(crate) fn apply(
        self,
        cell: &(impl HistoryCell + ?Sized),
        mut lines: Vec<HyperlinkLine>,
    ) -> Vec<HyperlinkLine> {
        if self.compact {
            // Only user cards have decorative padding. Trimming arbitrary cells would eat
            // paragraph and code-block boundaries when an answer arrives in stream chunks.
            if cell.type_id() == TypeId::of::<UserHistoryCell>() {
                if lines.first().is_some_and(is_blank) {
                    lines.remove(0);
                }
                if lines.last().is_some_and(is_blank) {
                    lines.pop();
                }
            }

            // Keep the tool header and the last result row (often an error or summary).
            // The transcript retains the original cell, so shortening this preview is lossless.
            // Four rows make repeated tool calls small without reducing them to opaque titles.
            if (cell.type_id() == TypeId::of::<ExecCell>()
                || cell.type_id() == TypeId::of::<McpToolCallCell>())
                && lines.len() > 4
                && let Some(last) = lines.pop()
            {
                lines.truncate(2);
                lines.push(HyperlinkLine::new("    … (see transcript)".dim().into()));
                lines.push(last);
            }
        }
        if self.large_bullets {
            for line in &mut lines {
                // Only column-zero markers belong to the UI. Bullets in Markdown, output,
                // and code are indented and must retain their original text and styling.
                if let Some(span) = line.line.spans.iter_mut().find(|s| !s.content.is_empty())
                    && let Some(rest) = span.content.strip_prefix('•')
                {
                    // Both markers occupy one terminal column, preserving hyperlink ranges.
                    span.content = format!("●{rest}").into();
                }
            }
        }
        lines
    }
}

fn is_blank(line: &HyperlinkLine) -> bool {
    line.line
        .spans
        .iter()
        .all(|span| span.content.trim().is_empty())
}

#[cfg(test)]
#[path = "presentation_tests.rs"]
mod tests;
