use pier_config::theme;
use pier_core::core::{Core, SubmitFocus};
use ratatui::{
	layout::{Constraint, Layout, Rect},
	style::{Modifier, Style},
	widgets::{Block, Borders, List, ListItem, Paragraph},
	Frame,
};
use super::super::centered_rect;

pub(crate) struct Submit;

impl Submit {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let overlay_area = centered_rect(70, 45, area);
		f.render_widget(ratatui::widgets::Clear, overlay_area);
		
		let chunks = Layout::default()
			.direction(ratatui::layout::Direction::Vertical)
			.constraints([
				Constraint::Length(3), // Description
				Constraint::Length(7), // File List (5 items + 2 borders)
			])
			.split(overlay_area);
			
		// Description Block
		let desc_style = if core.submit_focus == SubmitFocus::Description {
			Style::default().fg(theme().component.active_pane_border)
		} else {
			Style::default().fg(theme().component.pane_border)
		};
		let desc_title = format!("─Description ({}) ", core.submit_description.len());
		let desc_block = Block::default()
			.title(desc_title)
			.borders(Borders::ALL)
			.border_set(ratatui::symbols::border::ROUNDED)
			.border_style(desc_style);
		
		let p = Paragraph::new(core.submit_description.as_str()).block(desc_block).style(Style::default().fg(theme().component.default_text));
		f.render_widget(p, chunks[0]);
		
		// Add blinking cursor
		if core.submit_focus == SubmitFocus::Description {
			let cursor_x = chunks[0].x + 1 + core.submit_description.len() as u16;
			let cursor_y = chunks[0].y + 1;
			let max_x = chunks[0].x + chunks[0].width - 2;
			f.set_cursor_position((cursor_x.min(max_x), cursor_y));
		}
		
		// File List Block
		let list_style = if core.submit_focus == SubmitFocus::FileList {
			Style::default().fg(theme().component.active_pane_border)
		} else {
			Style::default().fg(theme().component.pane_border)
		};
		let list_block = Block::default()
			.title("─Files to Submit (tab to toggle view) ")
			.borders(Borders::ALL)
			.border_set(ratatui::symbols::border::ROUNDED)
			.border_style(list_style);
			
		let list_inner = chunks[1].inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
		f.render_widget(list_block, chunks[1]);
		
		let mut items = Vec::new();
		for (i, file) in core.pending_files.iter().enumerate() {
			let _is_selected = core.submit_cursor == i;
			let symbol = " "; 
			
			let (icon, color) = match file.action.as_str() {
				"add" => (&theme().icon.mark_add, theme().p4.add),
				"edit" => (&theme().icon.own_edit, theme().p4.edit),
				"delete" => (&theme().icon.mark_delete, theme().p4.delete),
				_ => (&theme().icon.file_default, theme().component.default_text),
			};
			
			let filename = std::path::Path::new(&file.path)
				.file_name()
				.map(|n| n.to_string_lossy().to_string())
				.unwrap_or_else(|| file.path.clone());
			
			let parent_path = std::path::Path::new(&file.path)
				.parent()
				.map(|p| p.to_string_lossy().to_string())
				.unwrap_or_default();
			let display_path = parent_path.replacen("//depot", "...", 1);
			
			// 缩进朝左移一个字符
			let left_content = format!("  {}{} {} ", symbol, icon, filename);
			let right_content = format!("{}  {}/ ", file.revision, display_path);
			
			let total_width = list_inner.width as usize;
			let left_len = left_content.chars().count();
			let right_len = right_content.chars().count();

			let mut line_spans = vec![ratatui::text::Span::styled(left_content, Style::default().fg(color))];
			
			if total_width > left_len {
				let avail_right = total_width.saturating_sub(left_len);
				let final_right = if right_len > avail_right {
					format!("{}...", &right_content[..avail_right.saturating_sub(3)])
				} else {
					let padding = avail_right.saturating_sub(right_len);
					format!("{}{}", " ".repeat(padding), right_content)
				};
				line_spans.push(ratatui::text::Span::styled(final_right, Style::default().fg(theme().component.pane_border)));
			}
			
			items.push(ListItem::new(ratatui::text::Line::from(line_spans)));
		}
		
		let list = List::new(items);
		let mut list_state = ratatui::widgets::ListState::default();
		list_state.select(Some(core.submit_cursor));
		
		let highlight_style = if core.submit_focus == SubmitFocus::FileList {
			Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
		} else {
			Style::default()
		};
		
		f.render_stateful_widget(list.highlight_style(highlight_style), list_inner, &mut list_state);
	}
}
