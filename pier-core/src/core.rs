use std::env;
use crate::filetree::FileTree;

#[derive(PartialEq, Clone, Copy)]
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
    pub filetree: FileTree,

	pub scope_panel: ActivePanel,
	pub filetree_panel: ActivePanel,
	pub pending_panel: ActivePanel,
	pub log_panel: ActivePanel,

	pub input: ActivePanel,
	pub confirm: ActivePanel,
}

impl Core {
    pub fn new() -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| env::home_dir().unwrap_or_default());
        Self {
            active_panel: ActivePanel::FileTree,
            filetree: FileTree::new(current_dir),

            scope_panel: ActivePanel::Scope,
            filetree_panel: ActivePanel::FileTree,
            pending_panel: ActivePanel::Pending,
            log_panel: ActivePanel::Log,
            input: ActivePanel::Input,
            confirm: ActivePanel::Confirm,
        }
    }
}
