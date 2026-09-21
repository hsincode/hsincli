use super::*;
use pretty_assertions::assert_eq;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

#[test]
fn status_markers_preserve_styles_separators_and_preview_text() {
    let mut snapshots = Vec::new();
    for style in [BulletStyle::Small, BulletStyle::Large] {
        let child = Paragraph::new(vec![
            Line::from(vec!["•".green(), " Working (0s • esc to interrupt)".into()]),
            Line::from("◦ Working (1s • esc to interrupt)".dim()),
            Line::from("• Queued follow-up inputs"),
            Line::from("  ↳ • user text"),
            Line::from("• Running hook"),
        ]);
        let area = Rect::new(
            /*x*/ 2, /*y*/ 1, /*width*/ 40, /*height*/ 5,
        );
        let surface = StatusBullets {
            child: RenderableItem::Owned(Box::new(child)),
            style,
        };
        let mut buf = Buffer::empty(area);
        surface.render(area, &mut buf);
        assert_eq!(surface.desired_height(area.width), 5);
        assert_eq!(buf[(area.x, area.y)].fg, ratatui::style::Color::Green);
        snapshots.push(
            buf.content
                .chunks(40)
                .map(|row| {
                    row.iter()
                        .map(ratatui::buffer::Cell::symbol)
                        .collect::<String>()
                        .trim_end()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    insta::assert_snapshot!(snapshots.join("\n---\n"));
}
