use pier_config::theme;
use pier_core::core::{ActivePanel, Core};
use pier_core::filetree::FileP4Status;
use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Modifier, Style},
	widgets::{List, ListItem},
	Frame,
};
use super::get_block;

pub(crate) struct FileTree;

impl FileTree {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let ft_block = get_block("[2] FileTree", ActivePanel::FileTree, core.active_panel);
		let ft_inner_area = ft_block.inner(area);
		f.render_widget(ft_block, area);

		// 增加左右 padding 确保 highlight 不贴边
		let ft_padded_area = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
			.split(ft_inner_area)[1];

		let ft_chunks = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([
				Constraint::Percentage(30),
				Constraint::Length(1), // 增加一个字符的间隔
				Constraint::Percentage(70)
			])
			.split(ft_padded_area);
		
		let parent_items: Vec<ListItem> = core.filetree.parent_files.iter().map(|file| {
			let (icon, color) = if file.is_dir {
				if file.path == core.client_root {
					(&theme().icon.client_root, theme().component.default_text)
				} else {
					(&theme().icon.folder_open, theme().component.default_text)
				}
			} else {
				match file.p4_status {
					FileP4Status::Add => (&theme().icon.mark_add, theme().p4.add),
					FileP4Status::Edit => (&theme().icon.own_edit, theme().p4.edit),
					FileP4Status::Delete => (&theme().icon.mark_delete, theme().p4.delete),
					FileP4Status::OtherCheckout => (&theme().icon.other_checkout, theme().p4.other_checkout),
					FileP4Status::Untracked => (&theme().icon.untracked, theme().component.default_text),
					_ => (&theme().icon.file_default, theme().component.default_text),
				}
			};
			ListItem::new(format!(" {} {} ", icon, file.name)).style(Style::default().fg(color))
		}).collect();
		
		let current_items: Vec<ListItem> = core.filetree.files.iter().enumerate().map(|(idx, file)| {
			let is_selected = core.filetree.selected == idx;
			let (icon, color) = if file.is_dir {
				if file.path == core.client_root {
					(&theme().icon.client_root, theme().component.default_text)
				} else if core.virtual_root.as_ref() == Some(&file.path) {
					(&theme().icon.virtual_root, theme().component.active_pane_border)
				} else if is_selected {
					(&theme().icon.folder_open, theme().component.default_text)
				} else if file.is_empty {
					(&theme().icon.folder_empty, theme().component.default_text)
				} else {
					(&theme().icon.folder, theme().component.default_text)
				}
			} else {
				match file.p4_status {
					FileP4Status::Add => (&theme().icon.mark_add, theme().p4.add),
					FileP4Status::Edit => (&theme().icon.own_edit, theme().p4.edit),
					FileP4Status::Delete => (&theme().icon.mark_delete, theme().p4.delete),
					FileP4Status::OtherCheckout => (&theme().icon.other_checkout, theme().p4.other_checkout),
					FileP4Status::Untracked => (&theme().icon.untracked, theme().component.default_text),
					_ => (&theme().icon.file_default, theme().component.default_text),
				}
			};
			
			// 如果未被高亮选中，在原本 ">" 的位置显示对应颜色的 1/2 宽实心块
			let status_block = if is_selected {
				"  " 
			} else {
				match file.p4_status {
					FileP4Status::Add | FileP4Status::Edit | FileP4Status::Delete | FileP4Status::OtherCheckout => "▌ ",
					_ => "  ",
				}
			};
			
			let block_style = if is_selected { Style::default() } else { Style::default().fg(color) };
			
			let line = ratatui::text::Line::from(vec![
				ratatui::text::Span::styled(status_block, block_style),
				ratatui::text::Span::styled(format!("{} {} ", icon, file.name), Style::default().fg(color))
			]);
			
			ListItem::new(line)
		}).collect();

		let is_ft_active = core.active_panel == ActivePanel::FileTree;

		let parent_highlight_style = if is_ft_active {
			Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
		} else {
			Style::default().add_modifier(Modifier::UNDERLINED)
		};

		let parent_list = List::new(parent_items)
			.style(Style::default().fg(theme().component.default_text))
			.highlight_style(parent_highlight_style)
			.highlight_symbol("");

		let current_highlight_style = if is_ft_active {
			Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
		} else {
			Style::default().add_modifier(Modifier::UNDERLINED)
		};

		let current_list = List::new(current_items)
			.style(Style::default().fg(theme().component.default_text))
			.highlight_style(current_highlight_style)
			.highlight_symbol("");

		let mut parent_list_state = ratatui::widgets::ListState::default();
		parent_list_state.select(Some(core.filetree.parent_selected));
		
		let mut current_list_state = ratatui::widgets::ListState::default();
		current_list_state.select(Some(core.filetree.selected));

		f.render_stateful_widget(parent_list, ft_chunks[0], &mut parent_list_state);
		f.render_stateful_widget(current_list, ft_chunks[2], &mut current_list_state);
	}
}
