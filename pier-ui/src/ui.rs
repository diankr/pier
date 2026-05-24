use pier_core::core::{ActivePanel, Core};
use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	widgets::{Block, Borders, List, ListItem, Paragraph},
	Frame,
};

pub struct UiState {
	pub is_scope_expanded: bool,
}

impl UiState {
	pub fn new() -> Self {
		Self {
			is_scope_expanded: false,
		}
	}
}

pub fn render_root(f: &mut Frame, area: Rect, state: &UiState, core: &Core) {
	let root_chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Min(1),
			Constraint::Length(1),
		])
		.split(area);

	let main_rect = root_chunks[0];
	let footer_area = root_chunks[1];

	let main_chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
		.split(main_rect);

	let left_area = main_chunks[0];
	let right_area = main_chunks[1];

	let scope_constraint = if state.is_scope_expanded {
		Constraint::Percentage(50)
	} else {
		Constraint::Length(3)
	};

	let left_chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			scope_constraint,
			Constraint::Percentage(70),
			Constraint::Min(3),
		])
		.split(left_area);

	let scope_area = left_chunks[0];
	let filetree_area = left_chunks[1];
	let pending_area = left_chunks[2];

	let right_chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Percentage(30),
			Constraint::Percentage(50),
			Constraint::Percentage(20),
		])
		.split(right_area);

	let changelist_area = right_chunks[0];
	let detail_area = right_chunks[1];
	let log_area = right_chunks[2];

	// Scope
	let scope_block = get_block("[1] Scope ", ActivePanel::Scope, core.active_panel);
	let scope_inner = scope_block.inner(scope_area);
	
	let root_str = core.client_root.to_string_lossy();
	let display_text = if scope_inner.width > 15 {
		let prefix = "Client Root: ";
		let full_text = format!("{}{}", prefix, root_str);
		
		if full_text.len() as u16 <= scope_inner.width {
			full_text
		} else {
			// 缩略逻辑：保留最后一段
			let last_part = core.client_root.file_name()
				.map(|n| n.to_string_lossy().to_string())
				.unwrap_or_else(|| root_str.to_string());
			
			let abbreviated = format!("{}.../{}", prefix, last_part);
			if abbreviated.len() as u16 <= scope_inner.width {
				abbreviated
			} else {
				// 如果缩略后还长，就暴力截断
				abbreviated.chars().take(scope_inner.width as usize).collect()
			}
		}
	} else {
		"".to_string()
	};

	f.render_widget(Paragraph::new(display_text).block(scope_block), scope_area);
	
	// FileTree (Single Outer Block)
	let ft_block = get_block("[2] FileTree ", ActivePanel::FileTree, core.active_panel);
	let ft_inner_area = ft_block.inner(filetree_area);
	f.render_widget(ft_block, filetree_area);

	let ft_chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
		.split(ft_inner_area);
	
	let parent_items: Vec<ListItem> = core.filetree.parent_files.iter().map(|f| {
		ListItem::new(format!(" {} ", f.name))
	}).collect();
	
	let current_items: Vec<ListItem> = core.filetree.files.iter().map(|f| {
		let prefix = if f.is_dir { " " } else { " " };
		ListItem::new(format!("{}{}", prefix, f.name))
	}).collect();

	let parent_list = List::new(parent_items)
		.highlight_style(Style::default().add_modifier(Modifier::DIM))
		.highlight_symbol("");

	let current_list = List::new(current_items)
		.highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
		.highlight_symbol("> ");

	let mut parent_list_state = ratatui::widgets::ListState::default();
	parent_list_state.select(Some(core.filetree.parent_selected));
	
	let mut current_list_state = ratatui::widgets::ListState::default();
	current_list_state.select(Some(core.filetree.selected));

	f.render_stateful_widget(parent_list, ft_chunks[0], &mut parent_list_state);
	f.render_stateful_widget(current_list, ft_chunks[1], &mut current_list_state);

	f.render_widget(get_block("[3] Pending ", ActivePanel::Pending, core.active_panel), pending_area);

	// Right Panels
	let cl_block = get_block("[4] ChangeList ", ActivePanel::ChangeList, core.active_panel);
	let mut cl_items: Vec<ListItem> = Vec::new();
	let mut current_ui_index = 0;
	let mut selectable_index = 0;
	let mut selected_ui_index = 0;

	// 计算可用宽度：
	// 总宽度 - 左右边框(2) - 前缀符号位(2) - 右边边距(1) = area.width - 5
	let content_width = (changelist_area.width as usize).saturating_sub(5);
	let is_cl_active = core.active_panel == ActivePanel::ChangeList;

	for (i, cl) in core.changelists.iter().enumerate() {
		let is_expanded = core.expanded_ids.contains(&cl.id);
		let is_head = i == 0;
		let is_selected = core.cl_cursor == selectable_index;
		
		// 1. 确定前缀符号 (2 字符)
		let symbol = if is_selected && is_cl_active {
			if is_head { "󰌕 " } else { "> " }
		} else {
			if is_head { "󰌕 " } else { "  " }
		};

		// 2. ID 行内容格式化
		let id_str = &cl.id;
		let author_str = format!("  {}", cl.author);
		let time_str = &cl.time;
		
		let id_len = id_str.len();
		let author_len = author_str.len();
		let time_len = time_str.len();
		
		let padding = content_width.saturating_sub(id_len).saturating_sub(author_len).saturating_sub(time_len);
		// 右侧增加一个空格
		let base_line = format!("{}{}{}{} ", id_str, author_str, " ".repeat(padding), time_str);
		
		cl_items.push(ListItem::new(format!("{}{}", symbol, base_line)));
		if is_selected {
			selected_ui_index = current_ui_index;
		}
		current_ui_index += 1;
		selectable_index += 1;

		if is_expanded {
			if let Some(details) = &cl.details {
				// 额外信息缩进 3 空格 (相对于 ID 行起始)
				// 前缀符号 2 + 缩进 3 = 5
				let detail_prefix = "     "; 
				let detail_content_width = content_width.saturating_sub(3);

				// 1. Submit 信息
				for desc_line in &details.full_description {
					cl_items.push(ListItem::new(format!("{}{}", detail_prefix, desc_line)).style(Style::default().fg(Color::Gray)));
					current_ui_index += 1;
				}

				// 2. 分割线
				let separator = "─".repeat(detail_content_width);
				cl_items.push(ListItem::new(format!("{}{}", detail_prefix, separator)).style(Style::default().fg(Color::DarkGray)));
				current_ui_index += 1;

				// 3. 文件列表
				for (_f_idx, file) in details.files.iter().enumerate() {
					let is_file_selected = core.cl_cursor == selectable_index;
					let file_symbol = if is_cl_active && is_file_selected { "> " } else { "  " };
					
					let file_prefix_str = format!("{}   ", file_symbol);
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

					cl_items.push(ListItem::new(file_line));
					if is_file_selected {
						selected_ui_index = current_ui_index;
					}
					current_ui_index += 1;
					selectable_index += 1;
				}
			}
		}
	}

	let highlight_style = if is_cl_active {
		Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
	} else {
		Style::default().add_modifier(Modifier::UNDERLINED)
	};

	let cl_list = List::new(cl_items).highlight_style(highlight_style);

	let mut cl_list_state = ratatui::widgets::ListState::default();
	cl_list_state.select(Some(selected_ui_index));

	f.render_stateful_widget(cl_list.block(cl_block), changelist_area, &mut cl_list_state);

	render_detail(f, detail_area, core);
	f.render_widget(get_block("[@] Log ", ActivePanel::Log, core.active_panel), log_area);

	// Footer
	let footer_text = format!(" [Q] Quit | [1-5] Switch Panel | Path: {}", core.filetree.current_path.display());
	let footer = Paragraph::new(footer_text)
		.style(Style::default().fg(Color::DarkGray));
	f.render_widget(footer, footer_area);
}
fn render_detail(f: &mut Frame, area: Rect, core: &Core) {
	let block = get_block("[Tab] Detail ", ActivePanel::Detail, core.active_panel);
	let inner = block.inner(area);
	f.render_widget(block, area);

	let is_dt_active = core.active_panel == ActivePanel::Detail;

	if let Some(detail) = &core.current_detail {
		let labels = [
			"FileName", "FileSize", "DepotPath", "Revision", 
			"DateModified", "ChangeList", "Action", "LatestUser", "CheckoutBy"
		];
		let values = [
			&detail.filename, &detail.filesize, &detail.depot_path, &detail.revision,
			&detail.date_modified, &detail.changelist, &detail.action, &detail.latest_user, &detail.checkout_by
		];

		let mut items = Vec::new();
		let content_width = (inner.width as usize).saturating_sub(4); // 减去 2(cursor) + 2(border)

		for (i, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
			let is_selected = is_dt_active && core.detail_cursor == i;
			let symbol = if is_selected { "> " } else { "  " };
			
			let padding = content_width.saturating_sub(label.len()).saturating_sub(value.len());
			let line = format!("{}{} {}{}", symbol, label, " ".repeat(padding), value);
			let mut list_item = ListItem::new(line);
			
			if is_selected {
				list_item = list_item.style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD));
			}
			items.push(list_item);
		}

		let list = List::new(items);
		f.render_widget(list, inner);
	} else if let Some(err) = &core.detail_error {
		let text = if err.contains("Not a Perforce-managed object") {
			"Not a Perforce-managed object"
		} else {
			err
		};
		let p = Paragraph::new(text)
			.style(Style::default().fg(Color::DarkGray))
			.alignment(ratatui::layout::Alignment::Center);
		
		// 垂直居中逻辑
		let vertical_chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints([
				Constraint::Percentage(45),
				Constraint::Min(1),
				Constraint::Percentage(45),
			])
			.split(inner);
		
		f.render_widget(p, vertical_chunks[1]);
	}
}

fn get_block(title: &'static str, panel: ActivePanel, active_panel: ActivePanel) -> Block<'static> {
	let style = if active_panel == panel {
		Style::default().fg(Color::Yellow)
	} else {
		Style::default().fg(Color::DarkGray)
	};
	Block::default()
		.title(title)
		.borders(Borders::ALL)
		.border_style(style)
}
