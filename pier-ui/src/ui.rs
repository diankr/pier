use pier_core::core::{ActivePanel, Core};
use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Style},
	widgets::{Block, Borders},
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
			Constraint::Length(2),
			Constraint::Min(1),
			Constraint::Length(2),
		])
		.split(area);

	let header = root_chunks[0];
	let main_rect = root_chunks[1];
	let footer = root_chunks[2];

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

	f.render_widget(Block::default().title("Header").borders(Borders::ALL), header);
	f.render_widget(Block::default().title("Footer").borders(Borders::ALL), footer);

	f.render_widget(get_block(" [1] Scope ", ActivePanel::Scope), scope_area);
	f.render_widget(
		get_block(" [2] FileTree ", ActivePanel::FileTree),
		filetree_area,
	);
	f.render_widget(get_block(" [3] Pending ", ActivePanel::Pending), pending_area);
	f.render_widget(get_block(" [4] Detail ", ActivePanel::Detail), detail_area);
	f.render_widget(get_block(" [5] Log ", ActivePanel::Log), log_area);
}
