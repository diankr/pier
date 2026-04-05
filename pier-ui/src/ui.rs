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
		.constraints([scope_constraint, Constraint::Min(0)])
		.split(left_area);

	let scope_area = left_chunks[0];
	let filetree_area = left_chunks[1];

	let right_chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Percentage(30),
			Constraint::Percentage(50),
			Constraint::Percentage(20),
		])
		.split(right_area);

	let pending_area = right_chunks[0];
	let detail_area = right_chunks[1];
	let log_area = right_chunks[2];

	let get_block = |title: &'static str, panel: ActivePanel| {
		let style = if core.active_panel == panel {
			Style::default().fg(Color::Yellow)
		} else {
			Style::default().fg(Color::DarkGray)
		};
		Block::default()
			.title(title)
			.borders(Borders::ALL)
			.border_style(style)
	};

	// Scope
	let scope_block = get_block("[1] Scope ", ActivePanel::Scope);
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
	let ft_block = get_block("[2] FileTree ", ActivePanel::FileTree);
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

	// Right Panels
	f.render_widget(get_block("[3] Pending ", ActivePanel::Pending), pending_area);
	f.render_widget(get_block("[4] Detail ", ActivePanel::Detail), detail_area);
	f.render_widget(get_block("[5] Log ", ActivePanel::Log), log_area);

	// Footer
	let footer_text = format!(" [Q] Quit | [1-5] Switch Panel | Path: {}", core.filetree.current_path.display());
	let footer = Paragraph::new(footer_text)
		.style(Style::default().fg(Color::DarkGray));
	f.render_widget(footer, footer_area);
}
