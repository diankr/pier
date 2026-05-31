use pier_config::theme;
use pier_core::core::{ActivePanel, Core};
use ratatui::{
	layout::Rect,
	style::Style,
	widgets::Paragraph,
	Frame,
};

pub(crate) struct Footer;

impl Footer {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let left_hints = match core.active_panel {
			ActivePanel::Scope      => "[Enter] p4 info",
			ActivePanel::FileTree   => "[c] checkout | [a] add | [d] delete | [r] revert",
			ActivePanel::Pending    => "[S] submit | [r] revert",
			ActivePanel::ChangeList => "[s] fetch & sync | [f] fetch | [g] sync to selected | [F] show in filetree",
			ActivePanel::Detail     => "[y] copy to clipboard",
			_ => "",
		};
		
		let right_fixed = "[Q] quit | [?] keybind | ver 0.0.1";
		let total_width = area.width as usize;
		let right_width = right_fixed.chars().count();
		
		let footer_line = if total_width > right_width + 5 {
			let avail_left = total_width.saturating_sub(right_width).saturating_sub(2);
			let left_part = if left_hints.chars().count() > avail_left {
				let mut s: String = left_hints.chars().take(avail_left.saturating_sub(3)).collect();
				s.push_str("...");
				s
			} else {
				left_hints.to_string()
			};
			let spacing = total_width.saturating_sub(left_part.chars().count()).saturating_sub(right_width);
			format!("{}{}{}", left_part, " ".repeat(spacing), right_fixed)
		} else {
			right_fixed.to_string()
		};

		let footer = Paragraph::new(footer_line)
			.style(Style::default().fg(theme().component.pane_border));
		f.render_widget(footer, area);
	}
}
