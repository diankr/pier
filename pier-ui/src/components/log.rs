use pier_config::theme;
use pier_core::core::{ActivePanel, Core};
use ratatui::{
	layout::Rect,
	style::{Modifier, Style},
	widgets::{Paragraph, Wrap},
	Frame,
};
use super::get_block;

pub(crate) struct Log;

impl Log {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let block = get_block("[@] Log", ActivePanel::Log, core.active_panel);
		
		let is_log_active = core.active_panel == ActivePanel::Log;

		// 重新实现 Log 渲染，使用 Paragraph 以支持 Wrap 和滚动
		let mut log_content = Vec::new();
		for (i, log) in core.logs.iter().enumerate() {
			let is_selected = is_log_active && core.log_cursor == i;
			let header_style = if is_selected {
				Style::default().fg(theme().component.active_pane_border).add_modifier(Modifier::BOLD)
			} else {
				Style::default().fg(theme().component.pane_border)
			};

			log_content.push(ratatui::text::Line::from(vec![
				ratatui::text::Span::styled(format!("[{}]", log.time), header_style)
			]));
			log_content.push(ratatui::text::Line::from(vec![
				ratatui::text::Span::styled(format!("> {}", log.command), Style::default().fg(theme().component.default_text))
			]));
			
			for line in log.output.lines() {
				log_content.push(ratatui::text::Line::from(vec![
					ratatui::text::Span::styled(format!("  {}", line), Style::default().fg(theme().component.pane_border))
				]));
			}
			log_content.push(ratatui::text::Line::from("")); // Spacer
		}

		let paragraph = Paragraph::new(log_content)
			.style(Style::default().fg(theme().component.default_text))
			.block(block)
			.wrap(Wrap { trim: true });
		
		// 更加精准的滚动：根据 log_cursor 前面的日志所占的行数计算 offset
		let mut scroll_offset = 0;
		for (i, log) in core.logs.iter().enumerate() {
			if i >= core.log_cursor { break; }
			let lines_count = log.output.lines().count() as u16;
			scroll_offset += 1 + 1 + lines_count + 1; // Time + Cmd + Output + Spacer
		}
		
		f.render_widget(paragraph.scroll((scroll_offset, 0)), area);
	}
}
