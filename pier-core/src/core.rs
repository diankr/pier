use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::{HashSet, HashMap};
use crate::filetree::{FileTree, FileP4Status};
use crate::changelist::{ChangeListItem, ChangeListFile, fetch_changelists, fetch_changelist_detail};
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LogItem {
  pub time: String,
  pub command: String,
  pub output: String,
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

  pub logs: Vec<LogItem>,
  pub log_cursor: usize,

  pub pending_files: Vec<ChangeListFile>,
  pub is_pending_expanded: bool,
  pub pending_cursor: usize,

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
    let _ = env::set_current_dir(&client_root);
    let changelists = fetch_changelists(&client_root)?;
    let logs = Self::load_logs();
    let log_cursor = logs.len().saturating_sub(1);

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
      logs,
      log_cursor,
      pending_files: Vec::new(),
      is_pending_expanded: true,
      pending_cursor: 0,
      scope_panel: ActivePanel::Scope,
      filetree_panel: ActivePanel::FileTree,
      pending_panel: ActivePanel::Pending,
      changelist_panel: ActivePanel::ChangeList,
      detail_panel: ActivePanel::Detail,
      log_panel: ActivePanel::Log,
      input: ActivePanel::Input,
      confirm: ActivePanel::Confirm,
    };
    core.refresh_all();
    Ok(core)
  }

  pub fn refresh_all(&mut self) {
    self.filetree.refresh();
    self.update_file_p4_statuses();
    self.update_pending_files();
    self.update_detail();
  }

  pub fn update_pending_files(&mut self) {
    let output = Command::new("p4")
      .arg("opened")
      .output();

    self.pending_files.clear();
    if let Ok(out) = output {
      let stdout = String::from_utf8_lossy(&out.stdout);
      for line in stdout.lines() {
        let parts: Vec<&str> = line.split(" - ").collect();
        if parts.len() >= 2 {
          let path_rev = parts[0];
          let action_part = parts[1];
          let action = action_part.split_whitespace().next().unwrap_or("").to_string();
          let sub_parts: Vec<&str> = path_rev.split('#').collect();
          if sub_parts.len() >= 2 {
            self.pending_files.push(ChangeListFile {
              path: sub_parts[0].to_string(),
              revision: format!("#{}", sub_parts[1]),
              action,
            });
          }
        }
      }
    }
  }

  fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().to_lowercase().replace('\\', "/")
  }

  pub fn update_file_p4_statuses(&mut self) {
    let output = Command::new("p4")
      .arg("opened")
      .output();

    let mut opened_map = HashMap::new();
    if let Ok(out) = output {
      let stdout = String::from_utf8_lossy(&out.stdout);
      for line in stdout.lines() {
        if let Some(hash_idx) = line.find('#') {
          let depot_path = &line[..hash_idx];
          if let Some(dash_idx) = line.find(" - ") {
            let action_part = &line[dash_idx + 3..];
            let action = action_part.split_whitespace().next().unwrap_or("");
            opened_map.insert(depot_path.to_string(), action.to_string());
          }
        }
      }
    }

    let mut fstat_cmd = Command::new("p4");
    fstat_cmd.arg("fstat").arg("-T").arg("clientFile,depotFile");
    let mut files_to_stat = false;
    for file in &self.filetree.files {
      if !file.is_dir {
        fstat_cmd.arg(&file.path);
        files_to_stat = true;
      }
    }

    if files_to_stat {
      if let Ok(out) = fstat_cmd.output() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut tracked_normalized = HashMap::new(); // Normalized path -> depot path
        let mut current_block: HashMap<String, String> = HashMap::new();

        for line in stdout.lines() {
          let trimmed = line.trim();
          if trimmed.is_empty() {
            if let (Some(cf), Some(df)) = (current_block.get("clientFile"), current_block.get("depotFile")) {
              tracked_normalized.insert(cf.to_lowercase().replace('\\', "/"), df.clone());
            }
            current_block.clear();
            continue;
          }
          if trimmed.starts_with("... ") {
            let parts: Vec<&str> = trimmed[4..].splitn(2, ' ').collect();
            if parts.len() == 2 {
              current_block.insert(parts[0].to_string(), parts[1].trim().to_string());
            }
          }
        }
        if let (Some(cf), Some(df)) = (current_block.get("clientFile"), current_block.get("depotFile")) {
          tracked_normalized.insert(cf.to_lowercase().replace('\\', "/"), df.clone());
        }

        for file in self.filetree.files.iter_mut() {
          if file.is_dir { continue; }
          let norm = Self::normalize_path(&file.path);
          if let Some(depot_path) = tracked_normalized.get(&norm) {
            if let Some(action) = opened_map.get(depot_path) {
              file.p4_status = match action.as_str() {
                "add" => FileP4Status::Add,
                "edit" => FileP4Status::Edit,
                "delete" => FileP4Status::Delete,
                _ => FileP4Status::None,
              };
            } else {
              file.p4_status = FileP4Status::None;
            }
          } else {
            file.p4_status = FileP4Status::Untracked;
          }
        }
      }
    }
  }

  pub fn add_log(&mut self, command: &str, output: &str) {
    let now = chrono::Local::now().format("%H:%M:%S %m/%d").to_string();
    self.logs.push(LogItem {
      time: now,
      command: command.to_string(),
      output: output.to_string(),
    });
    
    // 保留最大100条
    if self.logs.len() > 100 {
      self.logs.remove(0);
    }
    
    self.log_cursor = self.logs.len().saturating_sub(1);
    self.save_logs();
  }

  fn load_logs() -> Vec<LogItem> {
    dirs::cache_dir()
      .map(|d| d.join("pier/logs.json"))
      .and_then(|p| std::fs::read_to_string(p).ok())
      .and_then(|s| serde_json::from_str(&s).ok())
      .unwrap_or_default()
  }

  fn save_logs(&self) {
    if let Some(path) = dirs::cache_dir().map(|d| d.join("pier/logs.json")) {
      if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
      }
      if let Ok(json) = serde_json::to_string(&self.logs) {
        let _ = std::fs::write(path, json);
      }
    }
  }

  pub fn ft_p4_edit(&mut self) {
    if let Some(file) = self.filetree.files.get(self.filetree.selected) {
      if file.is_dir { return; }
      let output = Command::new("p4").arg("edit").arg(&file.path).output();
      self.handle_p4_output("p4 edit", output);
      self.refresh_all();
    }
  }

  pub fn ft_p4_delete(&mut self) {
    if let Some(file) = self.filetree.files.get(self.filetree.selected) {
      if file.is_dir { return; }
      let output = Command::new("p4").arg("delete").arg(&file.path).output();
      self.handle_p4_output("p4 delete", output);
      self.refresh_all();
      if self.filetree.selected >= self.filetree.files.len() && !self.filetree.files.is_empty() {
        self.filetree.selected = self.filetree.files.len() - 1;
      }
    }
  }

  pub fn ft_p4_revert(&mut self) {
    if let Some(file) = self.filetree.files.get(self.filetree.selected) {
      if file.is_dir { return; }
      let output = Command::new("p4").arg("revert").arg(&file.path).output();
      self.handle_p4_output("p4 revert", output);
      self.refresh_all();
    }
  }

  pub fn ft_p4_add(&mut self) {
    if let Some(file) = self.filetree.files.get(self.filetree.selected) {
      if file.is_dir { return; }
      let output = Command::new("p4").arg("add").arg(&file.path).output();
      self.handle_p4_output("p4 add", output);
      self.refresh_all();
    }
  }

  pub fn pd_p4_revert(&mut self) {
    if self.pending_cursor > 0 {
      let file_idx = self.pending_cursor - 1;
      if let Some(file) = self.pending_files.get(file_idx) {
        let output = Command::new("p4").arg("revert").arg(&file.path).output();
        self.handle_p4_output("p4 revert", output);
        self.refresh_all();
        if self.pending_cursor > self.pending_files.len() {
          self.pending_cursor = self.pending_files.len();
        }
      }
    }
  }

  fn handle_p4_output(&mut self, cmd_name: &str, output: std::io::Result<std::process::Output>) {
    match output {
      Ok(out) => {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let combined = if stderr.is_empty() {
          stdout.to_string()
        } else {
          format!("{}\nERROR: {}", stdout, stderr)
        };
        self.add_log(cmd_name, &combined);
      }
      Err(e) => {
        self.add_log(cmd_name, &format!("Execution failed: {}", e));
      }
    }
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
    self.refresh_all();
    self.pending_cursor = 0;
  }

  pub fn ft_leave_dir(&mut self) {
    self.filetree.leave_dir();
    self.refresh_all();
    self.pending_cursor = 0;
  }

  pub fn pd_move_down(&mut self) {
    let count = if self.is_pending_expanded { self.pending_files.len() + 1 } else { 1 };
    if self.pending_cursor < count - 1 {
      self.pending_cursor += 1;
    }
  }

  pub fn pd_move_up(&mut self) {
    if self.pending_cursor > 0 {
      self.pending_cursor -= 1;
    }
  }

  pub fn pd_expand(&mut self) {
    if self.pending_cursor == 0 {
      self.is_pending_expanded = true;
    }
  }

  pub fn pd_collapse(&mut self) {
    self.is_pending_expanded = false;
    self.pending_cursor = 0;
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

  pub fn log_move_down(&mut self) {
    if !self.logs.is_empty() && self.log_cursor < self.logs.len() - 1 {
      self.log_cursor += 1;
    }
  }

  pub fn log_move_up(&mut self) {
    if self.log_cursor > 0 {
      self.log_cursor -= 1;
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
          self.sync_cl_cursor_after_collapse(&id);
        }
      }
    }
  }

  fn get_cl_selectable_count(&self) -> usize {
    let mut count = 0;
    for cl in &self.changelists {
      count += 1; 
      if self.expanded_ids.contains(&cl.id) {
        if let Some(details) = &cl.details {
          count += details.files.len(); 
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
    let output = Command::new("p4").arg("info").output()
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
  File(String, usize), 
}
