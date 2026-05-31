use pier_config::theme;
use pier_core::core::{ActivePanel, Core};
use ratatui::{
	layout::Rect,
	style::Style,
	widgets::Paragraph,
	Frame,
};
use super::get_block;

pub(crate) struct Domain;

impl Domain {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let scope_block = get_block("[1] Domain", ActivePanel::Scope, core.active_panel);
		let scope_inner = scope_block.inner(area);

		// 增加左侧 1 字符 margin
		let scope_padded_area = ratatui::layout::Layout::default()
			.direction(ratatui::layout::Direction::Horizontal)
			.constraints([ratatui::layout::Constraint::Length(1), ratatui::layout::Constraint::Min(0)])
			.split(scope_inner)[1];

		let root_str = core.virtual_root.as_ref()
			.map(|vr| vr.to_string_lossy())
			.unwrap_or_else(|| core.client_root.to_string_lossy());

		let is_virtual = core.virtual_root.is_some();
		let prefix = if is_virtual { "Virtual Root: " } else { "Client Root: " };

		let display_text = if scope_padded_area.width > 15 {
			let full_text = format!("{}{}", prefix, root_str);

			if full_text.len() as u16 <= scope_padded_area.width {
				full_text
			} else {
				let last_part = if is_virtual {
					core.virtual_root.as_ref().and_then(|vr| vr.file_name())
				} else {
					core.client_root.file_name()
				}
				.map(|n| n.to_string_lossy().to_string())
				.unwrap_or_else(|| root_str.to_string());

				let abbreviated = format!("{}.../{}", prefix, last_part);
				if abbreviated.len() as u16 <= scope_padded_area.width {
					abbreviated
				} else {
					abbreviated.chars().take(scope_padded_area.width as usize).collect()
				}
			}
		} else {
			"".to_string()
		};

		f.render_widget(scope_block, area);
		f.render_widget(Paragraph::new(display_text).style(Style::default().fg(theme().component.default_text)), scope_padded_area);
	}
}
