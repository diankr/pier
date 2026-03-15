// use ratatui::layout::{Position, Rect};

#[derive(PartialEq)]
pub enum ActivePanel {
	Scope,
	FileTree,
	Pending,
	Detail,
	Log,
}

pub struct Core {
	pub active_panel: ActivePanel,

	pub scope: ActivePanel,
	pub filetree: ActivePanel,
	pub pending: ActivePanel,
	pub log: ActivePanel,

	pub input: ActivePanel,
	pub confirm: ActivePanel,
}
