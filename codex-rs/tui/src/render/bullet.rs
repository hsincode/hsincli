//! Shared marker sizing for history and transient status rows.

use super::renderable::Renderable;
use super::renderable::RenderableItem;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Default)]
pub(crate) enum BulletStyle {
    #[default]
    Small,
    Large,
}

impl BulletStyle {
    pub(crate) fn from_large_bullets(large_bullets: bool) -> Self {
        if large_bullets {
            Self::Large
        } else {
            Self::Small
        }
    }

    pub(crate) fn marker(self, marker: &str) -> &str {
        match (self, marker) {
            (Self::Large, "•") => "●",
            // Preserve the dim animation phase as a large hollow circle.
            (Self::Large, "◦") => "○",
            _ => marker,
        }
    }
}

/// Applies marker sizing only to status surfaces, excluding the editable composer.
pub(crate) struct StatusBullets<'a> {
    pub(crate) child: RenderableItem<'a>,
    pub(crate) style: BulletStyle,
}

impl Renderable for StatusBullets<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.child.render(area, buf);
        // Status widgets own column-zero markers. Restricting replacement to that column
        // leaves elapsed-time separators, indented previews, and user-authored text intact.
        // Both sizes occupy one column, so styles and layout measurements stay unchanged.
        let area = area.intersection(buf.area);
        if area.width == 0 {
            return;
        }
        for y in area.top()..area.bottom() {
            let cell = &mut buf[(area.x, y)];
            match self.style.marker(cell.symbol()) {
                "●" => {
                    cell.set_symbol("●");
                }
                "○" => {
                    cell.set_symbol("○");
                }
                _ => {}
            }
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.child.desired_height(width)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.child.cursor_pos(area)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.child.cursor_style(area)
    }
}

#[cfg(test)]
#[path = "bullet_tests.rs"]
mod tests;
