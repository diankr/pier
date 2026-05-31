use pier_config::theme;
use pier_core::core::Core;
use ratatui::{
	layout::{Constraint, Layout, Rect},
	style::{Color, Style},
	widgets::{Block, Borders, Paragraph},
	text::{Line, Span},
	Frame,
};
use super::super::centered_rect;

pub(crate) struct Login;

impl Login {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let mut overlay_area = centered_rect(60, 25, area);
		if overlay_area.height < 12 {
			overlay_area.height = 12.min(area.height);
			overlay_area.y = (area.height.saturating_sub(overlay_area.height)) / 2;
		}
		f.render_widget(ratatui::widgets::Clear, overlay_area);

		let chunks = Layout::default()
			.direction(ratatui::layout::Direction::Vertical)
			.constraints([
				Constraint::Length(3), // Login (password)
				Constraint::Min(6),    // Info
			])
			.split(overlay_area);

		// Upper Block: p4 login
		let login_block = Block::default()
			.title(Line::from("─p4 login ").alignment(ratatui::layout::Alignment::Center))
			.borders(Borders::ALL)
			.border_set(ratatui::symbols::border::ROUNDED)
			.border_style(Style::default().fg(theme().component.active_pane_border));

		let masked_password = "*".repeat(core.login_password.len());
		let p = Paragraph::new(masked_password).block(login_block).style(Style::default().fg(theme().component.default_text));
		f.render_widget(p, chunks[0]);

		// Cursor for password
		let cursor_x = chunks[0].x + 1 + core.login_password.len() as u16;
		let cursor_y = chunks[0].y + 1;
		let max_x = chunks[0].x + chunks[0].width - 2;
		f.set_cursor_position((cursor_x.min(max_x), cursor_y));

		// Lower Block: info
		let info_block = Block::default()
			.title(Line::from("─info ").alignment(ratatui::layout::Alignment::Center))
			.borders(Borders::ALL)
			.border_set(ratatui::symbols::border::ROUNDED)
			.border_style(Style::default().fg(theme().component.pane_border));
		
		let info_style = if core.login_info == "Password Invalid" {
			Style::default().fg(Color::Red)
		} else {
			Style::default().fg(theme().component.default_text)
		};

		let content_width = chunks[1].width.saturating_sub(2);
		let mut info_content = vec![
			Line::from(vec![Span::raw(" "), Span::styled(&core.login_info, info_style)]),
			Line::from("─".repeat(content_width as usize)).style(Style::default().fg(theme().component.pane_border)),
		];

		// user:value (left and right aligned with 1-char margin)
		let user_label = "user";
		let user_value = &core.login_user;
		let user_padding = content_width.saturating_sub(user_label.len() as u16).saturating_sub(user_value.len() as u16).saturating_sub(2);
		info_content.push(Line::from(vec![
			Span::raw(" "),
			Span::raw(user_label),
			Span::raw(" ".repeat(user_padding as usize)),
			Span::raw(user_value),
			Span::raw(" "),
		]));

		// target server:value
		let server_label = "target server";
		let server_value = &core.login_server;
		let server_padding = content_width.saturating_sub(server_label.len() as u16).saturating_sub(server_value.len() as u16).saturating_sub(2);
		info_content.push(Line::from(vec![
			Span::raw(" "),
			Span::raw(server_label),
			Span::raw(" ".repeat(server_padding as usize)),
			Span::raw(server_value),
			Span::raw(" "),
		]));

		let p_info = Paragraph::new(info_content).block(info_block).style(Style::default().fg(theme().component.default_text));
		f.render_widget(p_info, chunks[1]);
	}
}
