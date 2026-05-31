use pier_config::theme;
use pier_core::core::{ActivePanel, Core};
use ratatui::{
	layout::{Direction, Layout, Rect},
	style::{Modifier, Style},
	widgets::{List, ListItem},
	Frame,
};
use super::get_block;

pub(crate) struct Pending;

impl Pending {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let block = get_block("[3] Pending", ActivePanel::Pending, core.active_panel);
		let inner = block.inner(area);
		f.render_widget(block, area);

		// 增加左右 padding
		let padded_inner = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([ratatui::layout::Constraint::Length(1), ratatui::layout::Constraint::Min(1), ratatui::layout::Constraint::Length(1)])
			.split(inner)[1];

		let mut items = Vec::new();
		let is_pd_active = core.active_panel == ActivePanel::Pending;
		
		// Default Changelist Header
		let toggle_symbol = if core.is_pending_expanded { "v" } else { ">" };
		
		let header_icon_span = ratatui::text::Span::styled(&theme().icon.pending_default, Style::default());
		let header_line = ratatui::text::Line::from(vec![
			ratatui::text::Span::from(format!("{} ", toggle_symbol)),
			header_icon_span,
			ratatui::text::Span::from(" Default ")
		]);

		let header_style = if is_pd_active && core.pending_cursor == 0 {
			Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
		} else if !is_pd_active && core.pending_cursor == 0 {
			Style::default().add_modifier(Modifier::UNDERLINED)
		} else {
			Style::default().fg(theme().component.default_text)
		};
		
		items.push(ListItem::new(header_line).style(header_style));

		// Files
		if core.is_pending_expanded {
			for (i, file) in core.pending_files.iter().enumerate() {
				let cursor_idx = i + 1;
				let is_selected = core.pending_cursor == cursor_idx;
				let symbol = if is_pd_active && is_selected { " " } else { " " };
				
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
				
				// 增加缩进，恢复层级感
				let left_content = format!("  {}{} {} ", symbol, icon, filename);
				let right_content = format!("{}  {}/ ", file.revision, display_path);
				
				let left_len = left_content.chars().count();
				let right_len = right_content.chars().count();
				let total_width = padded_inner.width as usize;
				
				let mut line_spans = vec![
					ratatui::text::Span::styled(left_content, Style::default().fg(color))
				];
				
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
		}

		let list = List::new(items)
			.style(Style::default().fg(theme().component.default_text));
		
		// 处理选中状态的逻辑移到渲染这里
		let mut state = ratatui::widgets::ListState::default();
		state.select(Some(core.pending_cursor));

		let highlight_style = if is_pd_active {
			Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
		} else if core.pending_cursor != usize::MAX {
			Style::default().add_modifier(Modifier::UNDERLINED)
		} else {
			Style::default()
		};

		f.render_stateful_widget(list.highlight_style(highlight_style), padded_inner, &mut state);
	}
}
