// use ratatui::layout::{Position, Rect};

#[derive(PartialEq)]
pub enum ActivePanel {
	Scope,
	FileTree,
	Pending,
	Detail,
	Log,
    Input,
    Confirm,
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

impl Core {
    pub fn new() -> Self {
        Self {
            active_panel: ActivePanel::FileTree,
            scope: ActivePanel::Scope,
            filetree: ActivePanel::FileTree,
            pending: ActivePanel::Pending,
            log: ActivePanel::Log,
            input: ActivePanel::Input,
            confirm: ActivePanel::Confirm,
        }
    }
}
