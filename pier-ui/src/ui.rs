use ratatui::{
	Frame,
	layout::{
		Constraint, Direction, Layout, Rect
	},
	widgets::{Block, Borders, canvas::Context}
};

pub struct UiState {
	is_scope_expanded: bool
}

impl UiState {
	pub fn new()-> Self {
		Self {
			is_scope_expanded: false,
		}
	}
}

pub fn render_root(f:&mut Frame, area:Rect, state:&UiState) {
	let root_chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Length(1),
			Constraint::Min(1),
			Constraint::Length(1),
		])
		.split(area);
	
	let header    = root_chunks[0];
	let main_rect = root_chunks[1];
	let footer    = root_chunks[2];

	let main_chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([
			Constraint::Percentage(60),
			Constraint::Percentage(40),
		])
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
			Constraint::Min(0),
		])
		.split(left_area);

	let scope_area    = left_chunks[0];
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
	let detail_area  = right_chunks[1];
	let log_area     = right_chunks[2];

	f.render_widget(Block::default().title("Header").borders(Borders::ALL), header);
	f.render_widget(Block::default().title("Footer").borders(Borders::ALL), footer);

	f.render_widget(Block::default().title("Scope").borders(Borders::ALL), scope_area);
	f.render_widget(Block::default().title("FileTree").borders(Borders::ALL), filetree_area);

	f.render_widget(Block::default().title("Pending").borders(Borders::ALL), pending_area);
	f.render_widget(Block::default().title("Detail").borders(Borders::ALL), detail_area);
	f.render_widget(Block::default().title("Log").borders(Borders::ALL), log_area);
}






