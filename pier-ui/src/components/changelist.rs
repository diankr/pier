use pier_config::theme;
use pier_core::core::{ActivePanel, Core};
use ratatui::{
	layout::Rect,
	style::{Color, Modifier, Style},
	widgets::{List, ListItem},
	Frame,
};
use super::get_block;

pub(crate) struct ChangeList;

impl ChangeList {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let cl_block = get_block("[4] ChangeList", ActivePanel::ChangeList, core.active_panel);
		let mut cl_items: Vec<ListItem> = Vec::new();
		let mut current_ui_index = 0;
		let mut selectable_index = 0;
		let mut selected_ui_index = 0;

		let content_width = (area.width as usize).saturating_sub(5);

		for (i, cl) in core.changelists.iter().enumerate() {
			let is_expanded = core.expanded_ids.contains(&cl.id);
			let is_head = i == 0;
			let is_selected = core.cl_cursor == selectable_index;
			
			let is_new = if let Some(synced_id) = &core.synced_change_id {
				let cl_val = cl.id.parse::<i64>().unwrap_or(0);
				let synced_val = synced_id.parse::<i64>().unwrap_or(0);
				cl_val > synced_val
			} else {
				false
			};

			let base_style = if is_new {
				Style::default().fg(Color::Red)
			} else {
				Style::default().fg(theme().component.default_text)
			};

			let is_synced = if let Some(synced_id) = &core.synced_change_id {
				cl.id == *synced_id
			} else {
				false
			};

			let symbol = if is_synced {
				format!(" {}", theme().icon.changelist_head)
			} else if is_head {
				format!(" \u{f0e95}") // \uf0e9 for server head
			} else {
				"  ".to_string()
			};

			let icon_span = ratatui::text::Span::styled(symbol, base_style);

			let id_str = format!(" {} ", cl.id);
			let author_str = format!("  {}", cl.author);
			let time_str = &cl.time;
			
			let id_len = id_str.len();
			let author_len = author_str.len();
			let time_len = time_str.len();
			
			let padding = content_width.saturating_sub(id_len).saturating_sub(author_len).saturating_sub(time_len);
			
			cl_items.push(ListItem::new(ratatui::text::Line::from(vec![
				icon_span, 
				ratatui::text::Span::styled(id_str, base_style.add_modifier(Modifier::BOLD)),
				ratatui::text::Span::styled(author_str, base_style),
				ratatui::text::Span::styled(" ".repeat(padding), base_style),
				ratatui::text::Span::styled(time_str, base_style),
			])));
			if is_selected {
				selected_ui_index = current_ui_index;
			}
			current_ui_index += 1;
			selectable_index += 1;

			if is_expanded {
				if let Some(details) = &cl.details {
					let detail_prefix = "     "; 
					let detail_content_width = content_width.saturating_sub(3);

					for desc_line in &details.full_description {
						cl_items.push(ListItem::new(format!("{}{}", detail_prefix, desc_line)).style(Style::default().fg(theme().component.pane_border)));
						current_ui_index += 1;
					}

					let separator = "─".repeat(detail_content_width);
					cl_items.push(ListItem::new(format!("{}{}", detail_prefix, separator)).style(Style::default().fg(theme().component.pane_border)));
					current_ui_index += 1;

					for (_f_idx, file) in details.files.iter().enumerate() {
						let is_file_selected = core.cl_cursor == selectable_index;
						
						let file_prefix_str = "      ";
						let file_info = format!("{} | {} | ", file.revision, file.action);
						
						let display_path = file.path.replacen("//depot", "...", 1);
						let file_info_len = file_info.chars().count();
						
						let avail_path_width = detail_content_width.saturating_sub(file_info_len);
						let path_len = display_path.chars().count();
						
						let file_line = if path_len <= avail_path_width {
							let path_padding = avail_path_width.saturating_sub(path_len);
							format!("{}{}{}{}{} ", file_prefix_str, file_info, " ".repeat(path_padding), display_path, " ")
						} else {
							format!("{}{}{} ", file_prefix_str, file_info, display_path)
						};

						cl_items.push(ListItem::new(file_line).style(Style::default().fg(theme().component.default_text)));
						if is_file_selected {
							selected_ui_index = current_ui_index;
						}
						current_ui_index += 1;
						selectable_index += 1;
					}
				}
			}
		}

		let is_cl_active = core.active_panel == ActivePanel::ChangeList;
		let highlight_style = if is_cl_active {
			Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
		} else {
			Style::default().add_modifier(Modifier::UNDERLINED)
		};

		let cl_list = List::new(cl_items).highlight_style(highlight_style).style(Style::default().fg(theme().component.default_text));

		let mut cl_list_state = ratatui::widgets::ListState::default();
		cl_list_state.select(Some(selected_ui_index));

		f.render_stateful_widget(cl_list.block(cl_block), area, &mut cl_list_state);
	}
}
