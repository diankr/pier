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

#[derive(PartialEq, Clone, Copy)]
pub enum SubmitFocus {
  Description,
  FileList,
}

#[derive(Debug, Clone)]
pub struct SyncFileInfo {
  pub depot_path: String,
  pub local_path: String,
  pub size: u64,
  pub synced: u64,
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

  pub is_submit_overlay_open: bool,
  pub submit_description: String,
  pub submit_cursor: usize,
  pub submit_focus: SubmitFocus,

  pub scope_panel: ActivePanel,
  pub filetree_panel: ActivePanel,
  pub pending_panel: ActivePanel,
  pub changelist_panel: ActivePanel,
  pub detail_panel: ActivePanel,
  pub log_panel: ActivePanel,

  pub input: ActivePanel,
  pub confirm: ActivePanel,

  pub is_login_overlay_open: bool,
  pub login_password: String,
  pub login_info: String,
  pub login_user: String,
  pub login_server: String,

  pub is_syncing: bool,
  pub sync_progress: f64,
  pub sync_files: Vec<SyncFileInfo>,
  pub sync_total: usize,
  pub sync_current: usize,
  pub sync_total_bytes: u64,
  pub sync_synced_bytes: u64,

  pub synced_change_id: Option<String>,
}

impl Core {
  pub fn new() -> Result<Self, String> {
    let (client_root, login_user, login_server, mut needs_login) = Self::detect_p4_info()?;
    let _ = env::set_current_dir(&client_root);
    
    // Attempt to fetch changelists, but don't fail if it's a login issue
    let changelists = match fetch_changelists(&client_root) {
      Ok(cls) => cls,
      Err(e) => {
        if e.contains("please login again") || e.contains("password") {
          needs_login = true;
          Vec::new()
        } else {
          // If it's another error, we still might want to proceed if we need login
          // but if we are supposedly logged in and this fails, it's a real error.
          if needs_login {
            Vec::new()
          } else {
            return Err(e);
          }
        }
      }
    };

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
      is_submit_overlay_open: false,
      submit_description: String::new(),
      submit_cursor: 0,
      submit_focus: SubmitFocus::Description,
      scope_panel: ActivePanel::Scope,
      filetree_panel: ActivePanel::FileTree,
      pending_panel: ActivePanel::Pending,
      changelist_panel: ActivePanel::ChangeList,
      detail_panel: ActivePanel::Detail,
      log_panel: ActivePanel::Log,
      input: ActivePanel::Input,
      confirm: ActivePanel::Confirm,
      is_login_overlay_open: needs_login,
      login_password: String::new(),
      login_info: "Your session has expired, please login again".to_string(),
      login_user,
      login_server,

      is_syncing: false,
      sync_progress: 0.0,
      sync_files: Vec::new(),
      sync_total: 0,
      sync_current: 0,
      sync_total_bytes: 0,
      sync_synced_bytes: 0,

      synced_change_id: None,

    };
    if !needs_login {
      core.detect_synced_change();
      core.refresh_all();
    }
    Ok(core)
  }

  pub fn detect_synced_change(&mut self) {
    let output = Command::new("p4").arg("changes").arg("-m").arg("1").arg("#have").output();
    if let Ok(out) = output {
      let stdout = String::from_utf8_lossy(&out.stdout);
      if let Some(line) = stdout.lines().next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
          let synced_id = parts[1].to_string();
          self.synced_change_id = Some(synced_id.clone());
          
          // Align cursor to synced change if it exists in the current list
          if let Some(pos) = self.changelists.iter().position(|c| c.id == synced_id) {
            // This is simplified, only aligns if it's a top-level change
            // and no expansions are active, which is fine for initial sync
            self.cl_cursor = pos;
          }
        }
      }
    }
  }

  pub fn p4_sync_cl(&mut self, cl_id: &str) {
    let output = Command::new("p4").arg("sync").arg(format!("@{}", cl_id)).output();
    self.handle_p4_output("p4 sync", output);
    self.detect_synced_change();
    self.refresh_all();
  }

  pub fn p4_sync_latest(&mut self) {
    let output = Command::new("p4").arg("sync").output();
    self.handle_p4_output("p4 sync", output);
    self.detect_synced_change();
    self.refresh_all();
  }

  pub fn p4_login(&mut self) {
    use std::io::Write;
    let mut child = Command::new("p4")
      .arg("login")
      .stdin(std::process::Stdio::piped())
      .stdout(std::process::Stdio::piped())
      .stderr(std::process::Stdio::piped())
      .spawn()
      .expect("Failed to spawn p4 login");

    if let Some(mut stdin) = child.stdin.take() {
      stdin.write_all(self.login_password.as_bytes()).ok();
      stdin.write_all(b"\n").ok();
    }

    let output = child.wait_with_output().expect("Failed to read p4 login output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if stdout.contains("logged in") {
      self.is_login_overlay_open = false;
      self.login_password.clear();
      self.refresh_all();
    } else {
      self.login_info = "Password Invalid".to_string();
    }
    
    self.add_log("p4 login", &format!("{}\n{}", stdout, stderr));
  }

  pub fn refresh_all(&mut self) {
    self.filetree.refresh();
    self.update_file_p4_statuses();
    self.update_pending_files();
    self.update_detail();
    if let Ok(cls) = fetch_changelists(&self.client_root) {
      self.changelists = cls;
    }
  }

  pub fn update_pending_files(&mut self) {
    let output = Command::new("p4")
      .arg("opened")
      .output();

    self.pending_files.clear();
    if let Ok(out) = output {
      let stdout = String::from_utf8_lossy(&out.stdout);
      let stderr = String::from_utf8_lossy(&out.stderr);
      if stdout.contains("please login again") || stderr.contains("please login again") {
        self.is_login_overlay_open = true;
        return;
      }
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
    if self.is_login_overlay_open { return; }
    let mut paths = Vec::new();
    for file in self.filetree.files.iter().chain(self.filetree.parent_files.iter()) {
      if !file.is_dir {
        paths.push(file.path.clone());
      }
    }
    let statuses = fetch_file_statuses(&paths);
    for file in self.filetree.files.iter_mut().chain(self.filetree.parent_files.iter_mut()) {
      if file.is_dir { continue; }
      if let Some(status) = statuses.get(&file.path) {
        file.p4_status = status.clone();
      } else {
        file.p4_status = FileP4Status::Untracked;
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
    if self.is_login_overlay_open { return; }
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
    self.pending_cursor = 0;
  }

  pub fn ft_leave_dir(&mut self) {
    self.filetree.leave_dir();
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
            let file_idx = idx - current;
            return Some(ClTarget::File(cl.id.clone(), details.files[file_idx].path.clone()));
          }
          current += details.files.len();
        }
      }
    }
    None
  }

  pub fn cl_get_local_path(&self, depot_path: &str) -> Option<PathBuf> {
    let output = Command::new("p4").arg("where").arg(depot_path).output().ok()?;
    if output.status.success() {
      let stdout = String::from_utf8_lossy(&output.stdout);
      // p4 where output: //depot/path //client/path /local/path
      let parts: Vec<&str> = stdout.split_whitespace().collect();
      if parts.len() >= 3 {
        return Some(PathBuf::from(parts[parts.len() - 1]));
      }
    }
    None
  }

  pub fn jump_to_file(&mut self, local_path: &Path) -> bool {
    if !local_path.exists() {
      return false;
    }

    let parent = local_path.parent().unwrap_or(Path::new("/"));
    self.filetree.current_path = parent.to_path_buf();
    self.filetree.refresh();
    
    if let Some(pos) = self.filetree.files.iter().position(|f| f.path == local_path) {
      self.filetree.selected = pos;
      self.active_panel = ActivePanel::FileTree;
      self.update_detail();
      true
    } else {
      false
    }
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

  fn detect_p4_info() -> Result<(PathBuf, String, String, bool), String> {
    let output = Command::new("p4").arg("info").output()
      .map_err(|e| format!("Failed to execute p4 command: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    let needs_login = combined.contains("please login again") || combined.contains("password");
    
    let mut client_root = None;
    let mut user = String::new();
    let mut server = String::new();

    for line in combined.lines() {
      if line.starts_with("Client root: ") {
        let path_str = line.trim_start_matches("Client root: ").trim();
        if path_str != "null" && !path_str.is_empty() {
          client_root = Some(PathBuf::from(path_str));
        }
      } else if line.starts_with("User name: ") {
        user = line.trim_start_matches("User name: ").trim().to_string();
      } else if line.starts_with("Server address: ") {
        server = line.trim_start_matches("Server address: ").trim().to_string();
      }
    }

    let root = client_root.ok_or_else(|| "Could not find 'Client root' in p4 info output.".to_string())?;
    
    Ok((root, user, server, needs_login))
  }

  pub fn p4_submit(&mut self) {
    if self.submit_description.trim().is_empty() {
      self.add_log("p4 submit", "Error: Description cannot be empty");
      return;
    }
    let output = Command::new("p4")
      .arg("submit")
      .arg("-d")
      .arg(&self.submit_description)
      .output();
    
    self.handle_p4_output("p4 submit", output);
    self.submit_description.clear();
    self.is_submit_overlay_open = false;
    
    // 同步刷新 Changelist
    if let Ok(changelists) = fetch_changelists(&self.client_root) {
      self.changelists = changelists;
      self.cl_cursor = 0;
      self.expanded_ids.clear();
    }
    
    self.refresh_all();
  }
}

pub enum ClTarget {
  Id(String),
  File(String, String), // id, depot_path
}

pub fn fetch_file_statuses(paths: &[PathBuf]) -> HashMap<PathBuf, FileP4Status> {
  let mut opened_map = HashMap::new();
  let mut other_opened = HashMap::new();
  
  let p4_user = env::var("P4USER").unwrap_or_default();
  let p4_client = env::var("P4CLIENT").unwrap_or_default();

  if let Ok(out) = Command::new("p4").arg("opened").arg("-a").output() {
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
      if let Some(hash_idx) = line.find('#') {
        let depot_path = &line[..hash_idx];
        if let Some(dash_idx) = line.find(" - ") {
          let action_part = &line[dash_idx + 3..];
          let mut parts = action_part.split_whitespace();
          let action = parts.next().unwrap_or("");
          
          let line_suffix = action_part.to_string();
          let is_own = if !p4_user.is_empty() && !p4_client.is_empty() {
            line_suffix.contains(&format!("by {}@{}", p4_user, p4_client))
          } else {
            false
          };

          if is_own {
            opened_map.insert(depot_path.to_string(), action.to_string());
          } else {
            other_opened.insert(depot_path.to_string(), action.to_string());
          }
        }
      }
    }
  }

  let mut fstat_cmd = Command::new("p4");
  fstat_cmd.arg("fstat").arg("-T").arg("clientFile,depotFile");
  let mut files_to_stat = false;
  
  for path in paths {
    fstat_cmd.arg(path);
    files_to_stat = true;
  }

  let mut result = HashMap::new();

  if files_to_stat {
    if let Ok(out) = fstat_cmd.output() {
      let stdout = String::from_utf8_lossy(&out.stdout);
      let mut tracked_normalized = HashMap::new(); 
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

      for path in paths {
        let norm = path.to_string_lossy().to_lowercase().replace('\\', "/");
        if let Some(depot_path) = tracked_normalized.get(&norm) {
          if let Some(action) = opened_map.get(depot_path) {
            let status = match action.as_str() {
              "add" => FileP4Status::Add,
              "edit" => FileP4Status::Edit,
              "delete" => FileP4Status::Delete,
              _ => FileP4Status::None,
            };
            result.insert(path.clone(), status);
          } else if other_opened.contains_key(depot_path) {
            result.insert(path.clone(), FileP4Status::OtherCheckout);
          } else {
            result.insert(path.clone(), FileP4Status::None);
          }
        } else {
          result.insert(path.clone(), FileP4Status::Untracked);
        }
      }
    }
  }
  result
}
