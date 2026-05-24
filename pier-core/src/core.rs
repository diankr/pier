use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::collections::HashSet;
use crate::filetree::FileTree;
use crate::changelist::{ChangeListItem, fetch_changelists, fetch_changelist_detail};
use crate::detail::{FileDetail, fetch_file_detail};

#[derive(PartialEq, Clone, Copy)]
pub enum ActivePanel {
	Scope,
	FileTree,
	Pending,
	ChangeList,
	Detail,
	Log,
	Input,
	Confirm,
}

pub struct Core {
	pub active_panel: ActivePanel,
	pub filetree: FileTree,
	pub client_root: PathBuf,

	pub changelists: Vec<ChangeListItem>,
	pub expanded_ids: HashSet<String>,
	pub cl_cursor: usize,

	pub current_detail: Option<FileDetail>,
	pub detail_error: Option<String>,
	pub detail_cursor: usize,

	pub scope_panel: ActivePanel,
	pub filetree_panel: ActivePanel,
	pub pending_panel: ActivePanel,
	pub changelist_panel: ActivePanel,
	pub detail_panel: ActivePanel,
	pub log_panel: ActivePanel,

	pub input: ActivePanel,
	pub confirm: ActivePanel,
}

impl Core {
	pub fn new() -> Result<Self, String> {
		let client_root = Self::detect_p4_root()?;
		
		// 自动 cd 到 client root
		let _ = env::set_current_dir(&client_root);

		let changelists = fetch_changelists(&client_root)?;

		let mut core = Self {
			active_panel: ActivePanel::FileTree,
			filetree: FileTree::new(client_root.clone()),
			client_root,

			changelists,
			expanded_ids: HashSet::new(),
			cl_cursor: 0,

			current_detail: None,
			detail_error: None,
			detail_cursor: 0,

			scope_panel: ActivePanel::Scope,
			filetree_panel: ActivePanel::FileTree,
			pending_panel: ActivePanel::Pending,
			changelist_panel: ActivePanel::ChangeList,
			detail_panel: ActivePanel::Detail,
			log_panel: ActivePanel::Log,
			input: ActivePanel::Input,
			confirm: ActivePanel::Confirm,
		};
		core.update_detail();
		Ok(core)
	}

	pub fn update_detail(&mut self) {
		if let Some(file) = self.filetree.files.get(self.filetree.selected) {
			match fetch_file_detail(&file.path) {
				Ok(detail) => {
					self.current_detail = Some(detail);
					self.detail_error = None;
				}
				Err(e) => {
					self.current_detail = None;
					self.detail_error = Some(e);
				}
			}
		} else {
			self.current_detail = None;
			self.detail_error = None;
		}
	}

	pub fn ft_move_down(&mut self) {
		self.filetree.move_down();
		self.update_detail();
	}

	pub fn ft_move_up(&mut self) {
		self.filetree.move_up();
		self.update_detail();
	}

	pub fn ft_enter_dir(&mut self) {
		self.filetree.enter_dir();
		self.update_detail();
	}

	pub fn ft_leave_dir(&mut self) {
		self.filetree.leave_dir();
		self.update_detail();
	}

	pub fn dt_move_down(&mut self) {
		if self.current_detail.is_some() && self.detail_cursor < 8 {
			self.detail_cursor += 1;
		}
	}

	pub fn dt_move_up(&mut self) {
		if self.detail_cursor > 0 {
			self.detail_cursor -= 1;
		}
	}

	pub fn dt_copy_selected(&self) {
		if let Some(detail) = &self.current_detail {
			let values = [
				&detail.filename, &detail.filesize, &detail.depot_path, &detail.revision,
				&detail.date_modified, &detail.changelist, &detail.action, &detail.latest_user, &detail.checkout_by
			];
			if let Some(val) = values.get(self.detail_cursor) {
				let _ = crate::detail::copy_to_clipboard(val);
			}
		}
	}

	pub fn cl_move_down(&mut self) {
		let count = self.get_cl_selectable_count();
		if count > 0 && self.cl_cursor < count - 1 {
			self.cl_cursor += 1;
		}
	}

	pub fn cl_move_up(&mut self) {
		if self.cl_cursor > 0 {
			self.cl_cursor -= 1;
		}
	}

	pub fn cl_expand(&mut self) {
		if let Some(target) = self.get_cl_target_at(self.cl_cursor) {
			if let ClTarget::Id(id) = target {
				if !self.expanded_ids.contains(&id) {
					self.expanded_ids.insert(id.clone());
					if let Some(cl) = self.changelists.iter_mut().find(|c| c.id == id) {
						if cl.details.is_none() {
							if let Ok(details) = fetch_changelist_detail(&id, &self.client_root) {
								cl.details = Some(details);
							}
						}
					}
					// 展开后更新缓存
					crate::changelist::save_to_cache(&self.changelists);
				}
			}
		}
	}

	pub fn cl_collapse(&mut self) {
		if let Some(target) = self.get_cl_target_at(self.cl_cursor) {
			let id_to_remove = match target {
				ClTarget::Id(id) => Some(id),
				ClTarget::File(id, _) => Some(id),
			};

			if let Some(id) = id_to_remove {
				if self.expanded_ids.contains(&id) {
					self.expanded_ids.remove(&id);
					// 收起后，如果光标在文件上，需要将光标移回对应的 ID 行
					self.sync_cl_cursor_after_collapse(&id);
				}
			}
		}
	}

	fn get_cl_selectable_count(&self) -> usize {
		let mut count = 0;
		for cl in &self.changelists {
			count += 1; // ID 行
			if self.expanded_ids.contains(&cl.id) {
				if let Some(details) = &cl.details {
					count += details.files.len(); // 文件行
				}
			}
		}
		count
	}

	pub fn get_cl_target_at(&self, idx: usize) -> Option<ClTarget> {
		let mut current = 0;
		for cl in &self.changelists {
			if current == idx {
				return Some(ClTarget::Id(cl.id.clone()));
			}
			current += 1;
			if self.expanded_ids.contains(&cl.id) {
				if let Some(details) = &cl.details {
					if idx < current + details.files.len() {
						return Some(ClTarget::File(cl.id.clone(), idx - current));
					}
					current += details.files.len();
				}
			}
		}
		None
	}

	fn sync_cl_cursor_after_collapse(&mut self, collapsed_id: &str) {
		let mut current_selectable_idx = 0;
		for cl in &self.changelists {
			if cl.id == collapsed_id {
				self.cl_cursor = current_selectable_idx;
				return;
			}
			current_selectable_idx += 1;
			if self.expanded_ids.contains(&cl.id) {
				if let Some(details) = &cl.details {
					current_selectable_idx += details.files.len();
				}
			}
		}
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

pub enum ClTarget {
	Id(String),
	File(String, usize), // cl_id, file_index
}
