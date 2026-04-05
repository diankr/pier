use std::env;
use std::path::PathBuf;
use std::process::Command;
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
	pub client_root: PathBuf,

	pub scope_panel: ActivePanel,
	pub filetree_panel: ActivePanel,
	pub pending_panel: ActivePanel,
	pub log_panel: ActivePanel,

	pub input: ActivePanel,
	pub confirm: ActivePanel,
}

impl Core {
	pub fn new() -> Result<Self, String> {
		let client_root = Self::detect_p4_root()?;
		
		// 自动 cd 到 client root
		let _ = env::set_current_dir(&client_root);

		Ok(Self {
			active_panel: ActivePanel::FileTree,
			filetree: FileTree::new(client_root.clone()),
			client_root,

			scope_panel: ActivePanel::Scope,
			filetree_panel: ActivePanel::FileTree,
			pending_panel: ActivePanel::Pending,
			log_panel: ActivePanel::Log,
			input: ActivePanel::Input,
			confirm: ActivePanel::Confirm,
		})
	}

	fn detect_p4_root() -> Result<PathBuf, String> {
		let output = Command::new("p4")
			.arg("info")
			.output()
			.map_err(|e| format!("Failed to execute p4 command: {}", e))?;

		if !output.status.success() {
			return Err("Not in a Perforce workspace (p4 info failed).".to_string());
		}

		let stdout = String::from_utf8_lossy(&output.stdout);
		for line in stdout.lines() {
			if line.starts_with("Client root: ") {
				let path_str = line.trim_start_matches("Client root: ").trim();
				if path_str == "null" || path_str.is_empty() {
					return Err("Perforce client root is not set (null).".to_string());
				}
				return Ok(PathBuf::from(path_str));
			}
		}

		Err("Could not find 'Client root' in p4 info output. Are you logged in?".to_string())
	}
}
