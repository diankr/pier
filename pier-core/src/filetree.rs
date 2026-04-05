use std::path::{Path, PathBuf};
use std::fs;

#[derive(Clone, Debug)]
pub struct FileItem {
	pub name: String,
	pub path: PathBuf,
	pub is_dir: bool,
}

pub struct FileTree {
	pub current_path: PathBuf,
	pub files: Vec<FileItem>,
	pub selected: usize,
	
	// 用于左侧列显示父级目录
	pub parent_files: Vec<FileItem>,
	pub parent_selected: usize,
}

impl FileTree {
	pub fn new(path: PathBuf) -> Self {
		let mut s = Self {
			current_path: path,
			files: Vec::new(),
			selected: 0,
			parent_files: Vec::new(),
			parent_selected: 0,
		};
		s.refresh();
		s
	}

	pub fn refresh(&mut self) {
		self.files = self.read_dir(&self.current_path);
		if let Some(parent) = self.current_path.parent() {
			self.parent_files = self.read_dir(parent);
			// 找到当前目录在父目录中的索引，以便高亮
			self.parent_selected = self.parent_files.iter()
				.position(|f| f.path == self.current_path)
				.unwrap_or(0);
		} else {
			self.parent_files = Vec::new();
		}
	}

	fn read_dir(&self, path: &Path) -> Vec<FileItem> {
		let mut items = Vec::new();
		if let Ok(entries) = fs::read_dir(path) {
			for entry in entries.flatten() {
				let path = entry.path();
				let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
				items.push(FileItem {
					name,
					is_dir: path.is_dir(),
					path,
				});
			}
		}
		// 排序：目录在前，文件名在后
		items.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
		items
	}

	pub fn move_down(&mut self) {
		if !self.files.is_empty() && self.selected < self.files.len() - 1 {
			self.selected += 1;
		}
	}

	pub fn move_up(&mut self) {
		if self.selected > 0 {
			self.selected -= 1;
		}
	}

	pub fn enter_dir(&mut self) {
		if let Some(file) = self.files.get(self.selected) {
			if file.is_dir {
				self.current_path = file.path.clone();
				self.selected = 0;
				self.refresh();
			}
		}
	}

	pub fn leave_dir(&mut self) {
		if let Some(parent) = self.current_path.parent() {
			let old_path = self.current_path.clone();
			self.current_path = parent.to_path_buf();
			self.refresh();
			// 选回之前的目录
			self.selected = self.files.iter()
				.position(|f| f.path == old_path)
				.unwrap_or(0);
		}
	}
}
