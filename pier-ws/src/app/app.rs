use std::{
	io::{self, Stdout},
	time::Duration,
	path::PathBuf,
	fs,
};

use anyhow::Result;
use crossterm::{
	event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pier_core::core::ActivePanel;
use pier_core::core::Core;
use pier_ui::ui::{render_root, UiState};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::time::{sleep, interval};
use tokio::io::AsyncBufReadExt;

use super::commands::Quit;

pub type Term = Terminal<CrosstermBackend<Stdout>>;

enum SyncEvent {
	Start(Vec<pier_core::core::SyncFileInfo>),
	FileDone(String),
	ByteProgress(Vec<u64>),
	End,
	Error(String),
}

pub(crate) struct App {
	pub(crate) core: Core,
	pub(crate) term: Term,
	pub(crate) state: UiState,
	pub(crate) should_quit: bool,
	detail_tx: tokio::sync::mpsc::UnboundedSender<PathBuf>,
	detail_rx: tokio::sync::mpsc::UnboundedReceiver<Result<pier_core::detail::FileDetail, String>>,
	status_tx: tokio::sync::mpsc::UnboundedSender<Vec<PathBuf>>,
	status_rx: tokio::sync::mpsc::UnboundedReceiver<std::collections::HashMap<PathBuf, pier_core::filetree::FileP4Status>>,
	
	sync_tx: tokio::sync::mpsc::UnboundedSender<SyncEvent>,
	sync_rx: tokio::sync::mpsc::UnboundedReceiver<SyncEvent>,
	sync_handle: Option<tokio::task::JoinHandle<()>>,
	sync_cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,

	last_cl_refresh: std::time::Instant,
}

impl App {
	fn new() -> Result<Self> {
		let core = Core::new().map_err(|e| anyhow::anyhow!(e))?;
		let backend = CrosstermBackend::new(io::stdout());
		let term = Terminal::new(backend)?;

		let (request_tx, mut request_rx) = tokio::sync::mpsc::unbounded_channel::<PathBuf>();
		let (result_tx, result_rx) = tokio::sync::mpsc::unbounded_channel::<Result<pier_core::detail::FileDetail, String>>();

		// Background worker
		tokio::spawn(async move {
			let mut last_path = None;
			while let Some(path) = request_rx.recv().await {
				last_path = Some(path.clone());
				tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
				if request_rx.len() > 0 {
					continue;
				}
				let result = pier_core::detail::fetch_file_detail(&path);
				let _ = result_tx.send(result);
			}
		});

		let (status_request_tx, mut status_request_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<PathBuf>>();
		let (status_result_tx, status_result_rx) = tokio::sync::mpsc::unbounded_channel::<std::collections::HashMap<PathBuf, pier_core::filetree::FileP4Status>>();

		tokio::spawn(async move {
			let mut last_paths = None;
			while let Some(paths) = status_request_rx.recv().await {
				last_paths = Some(paths.clone());
				tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
				if status_request_rx.len() > 0 {
					continue;
				}
				let statuses = pier_core::core::fetch_file_statuses(&paths);
				let _ = status_result_tx.send(statuses);
			}
		});

		let (sync_tx, sync_rx) = tokio::sync::mpsc::unbounded_channel::<SyncEvent>();

		Ok(Self {
			core,
			term,
			state: UiState::new(),
			should_quit: false,
			detail_tx: request_tx,
			detail_rx: result_rx,
			status_tx: status_request_tx,
			status_rx: status_result_rx,
			sync_tx,
			sync_rx,
			sync_handle: None,
			sync_cancel_tx: None,
			last_cl_refresh: std::time::Instant::now(),
		})
	}

	pub(crate) async fn serve() -> Result<()> {
		let mut app = Self::new()?;
		app.setup_terminal()?;
		let result = app.run().await;
		app.restore_terminal()?;
		result
	}

	fn setup_terminal(&mut self) -> Result<()> {
		enable_raw_mode()?;
		execute!(io::stdout(), EnterAlternateScreen)?;
		self.term.hide_cursor()?;
		Ok(())
	}

	fn restore_terminal(&mut self) -> Result<()> {
		disable_raw_mode()?;
		execute!(io::stdout(), LeaveAlternateScreen)?;
		self.term.show_cursor()?;
		Ok(())
	}

	async fn run(&mut self) -> Result<()> {
		self.trigger_detail_update();

		loop {
			if self.should_quit {
				break;
			}

			while let Ok(result) = self.detail_rx.try_recv() {
				match result {
					Ok(detail) => {
						self.core.current_detail = Some(detail);
						self.core.detail_error = None;
					}
					Err(e) => {
						self.core.current_detail = None;
						self.core.detail_error = Some(e);
					}
				}
			}

			while let Ok(statuses) = self.status_rx.try_recv() {
				for file in self.core.filetree.files.iter_mut().chain(self.core.filetree.parent_files.iter_mut()) {
					if file.is_dir { continue; }
					if let Some(status) = statuses.get(&file.path) {
						file.p4_status = status.clone();
					} else {
						file.p4_status = pier_core::filetree::FileP4Status::Untracked;
					}
				}
			}

			while let Ok(event) = self.sync_rx.try_recv() {
				match event {
					SyncEvent::Start(files) => {
						self.core.sync_total_bytes = files.iter().map(|f| f.size).sum();
						self.core.sync_files = files;
						self.core.sync_total = self.core.sync_files.len();
						self.core.sync_current = 0;
						self.core.sync_synced_bytes = 0;
						self.core.sync_progress = 0.0;
					}
					SyncEvent::FileDone(_file) => {
						self.core.sync_current += 1;
					}
					SyncEvent::ByteProgress(progress_vec) => {
						self.core.sync_synced_bytes = progress_vec.iter().sum();
						for file in self.core.sync_files.iter_mut() {
							if let Some(&synced) = progress_vec.get(file.original_index) {
								file.synced = synced;
							}
						}
						
						// Stable sort: files with progress (synced > 0) move to top
						self.core.sync_files.sort_by(|a, b| {
							let a_started = a.synced > 0;
							let b_started = b.synced > 0;
							b_started.cmp(&a_started)
						});

						if self.core.sync_total_bytes > 0 {
							self.core.sync_progress = self.core.sync_synced_bytes as f64 / self.core.sync_total_bytes as f64;
						}
					}
					SyncEvent::End => {
						self.core.is_syncing = false;
						self.core.detect_synced_change();
						self.sync_handle = None;
						self.sync_cancel_tx = None;
					}
					SyncEvent::Error(e) => {
						self.core.is_syncing = false;
						self.core.add_log("Sync Error", &e);
						self.sync_handle = None;
						self.sync_cancel_tx = None;
					}
				}
			}

			if self.last_cl_refresh.elapsed() >= Duration::from_secs(60) {
				if let Ok(cls) = pier_core::changelist::fetch_changelists(&self.core.client_root, self.core.virtual_root.as_deref()) {
					self.core.changelists = cls;
				}
				self.last_cl_refresh = std::time::Instant::now();
			}

			self.term.draw(|f| {
				let area = f.area();
				render_root(f, area, &self.state, &self.core);
			})?;

			if event::poll(Duration::from_millis(10))? {
				if let Event::Key(key) = event::read()? {
					if key.kind == KeyEventKind::Press {
						if self.core.is_syncing {
							if key.code == KeyCode::Esc {
								if let Some(cancel) = self.sync_cancel_tx.take() {
									let _ = cancel.send(());
								}
								self.core.is_syncing = false;
							}
						} else if self.core.is_info_overlay_open {
							self.handle_info_keys(key);
						} else if self.core.is_login_overlay_open {
							self.handle_login_keys(key);
						} else if self.core.is_submit_overlay_open {
							self.handle_submit_keys(key);
						} else {
							self.handle_main_keys(key);
						}
					}
				}
			}
			sleep(Duration::from_millis(10)).await;
		}
		Ok(())
	}

	fn handle_login_keys(&mut self, key: event::KeyEvent) {
		match (key.code, key.modifiers) {
			(KeyCode::Esc, _) => self.should_quit = true,
			(KeyCode::Enter, _) => {
				self.core.p4_login();
			}
			(KeyCode::Char(c), _) => {
				self.core.login_password.push(c);
			}
			(KeyCode::Backspace, _) => {
				self.core.login_password.pop();
			}
			_ => {}
		}
	}

	fn handle_info_keys(&mut self, key: event::KeyEvent) {
		match (key.code, key.modifiers) {
			(KeyCode::Esc, _) => self.core.is_info_overlay_open = false,
			(KeyCode::Tab, _) => {
				self.core.info_focus = match self.core.info_focus {
					pier_core::core::InfoFocus::Roots => pier_core::core::InfoFocus::Details,
					pier_core::core::InfoFocus::Details => pier_core::core::InfoFocus::Roots,
				};
			}
			(KeyCode::Char('j'), _) => {
				match self.core.info_focus {
					pier_core::core::InfoFocus::Roots => {
						let max = if self.core.is_roots_expanded { self.core.virtual_root_history.len() } else { 0 };
						if self.core.info_roots_cursor < max {
							self.core.info_roots_cursor += 1;
						}
					}
					pier_core::core::InfoFocus::Details => {
						if self.core.info_details_cursor < self.core.info_details.len().saturating_sub(1) {
							self.core.info_details_cursor += 1;
						}
					}
				}
			}
			(KeyCode::Char('k'), _) => {
				match self.core.info_focus {
					pier_core::core::InfoFocus::Roots => {
						if self.core.info_roots_cursor > 0 {
							self.core.info_roots_cursor -= 1;
						}
					}
					pier_core::core::InfoFocus::Details => {
						if self.core.info_details_cursor > 0 {
							self.core.info_details_cursor -= 1;
						}
					}
				}
			}
			(KeyCode::Char('l'), _) if self.core.info_focus == pier_core::core::InfoFocus::Roots => {
				self.core.is_roots_expanded = true;
			}
			(KeyCode::Char('h'), _) if self.core.info_focus == pier_core::core::InfoFocus::Roots => {
				self.core.is_roots_expanded = false;
				self.core.info_roots_cursor = 0;
			}
			(KeyCode::Enter, _) if self.core.info_focus == pier_core::core::InfoFocus::Roots => {
				let target_path = if self.core.info_roots_cursor == 0 {
					Some(self.core.client_root.clone())
				} else if let Some(vr) = self.core.virtual_root_history.get(self.core.info_roots_cursor - 1) {
					Some(vr.clone())
				} else {
					None
				};

				if let Some(path) = target_path {
					if self.core.info_roots_cursor == 0 {
						self.core.virtual_root = None;
					} else {
						self.core.virtual_root = Some(path.clone());
					}
					self.core.save_config();
					self.core.enter_path(&path);
					self.core.is_info_overlay_open = false;
					// Refresh changelists for the new virtual root
					if let Ok(cls) = pier_core::changelist::fetch_changelists(&self.core.client_root, self.core.virtual_root.as_deref()) {
						self.core.changelists = cls;
						self.core.cl_cursor = 0;
						self.core.expanded_ids.clear();
					}
				}
			}
			(KeyCode::Char('y'), _) if self.core.info_focus == pier_core::core::InfoFocus::Details => {
				if let Some((_, v)) = self.core.info_details.get(self.core.info_details_cursor) {
					use arboard::Clipboard;
					if let Ok(mut clipboard) = Clipboard::new() {
						let _ = clipboard.set_text(v.clone());
					}
				}
			}
			_ => {}
		}
	}

	fn handle_submit_keys(&mut self, key: event::KeyEvent) {
		match (key.code, key.modifiers) {
			(KeyCode::Esc, _) => self.core.is_submit_overlay_open = false,
			(KeyCode::Tab, _) => {
				self.core.submit_focus = match self.core.submit_focus {
					pier_core::core::SubmitFocus::Description => pier_core::core::SubmitFocus::FileList,
					pier_core::core::SubmitFocus::FileList => pier_core::core::SubmitFocus::Description,
				};
			}
			(KeyCode::Char('j'), _) if self.core.submit_focus == pier_core::core::SubmitFocus::FileList => {
				if self.core.submit_cursor < self.core.pending_files.len().saturating_sub(1) {
					self.core.submit_cursor += 1;
				}
			}
			(KeyCode::Char('k'), _) if self.core.submit_focus == pier_core::core::SubmitFocus::FileList => {
				if self.core.submit_cursor > 0 {
					self.core.submit_cursor -= 1;
				}
			}
			(KeyCode::Enter, _) => {
				self.core.p4_submit();
			}

			(KeyCode::Char(c), _) if self.core.submit_focus == pier_core::core::SubmitFocus::Description => {
				self.core.submit_description.push(c);
			}
			(KeyCode::Backspace, _) if self.core.submit_focus == pier_core::core::SubmitFocus::Description => {
				self.core.submit_description.pop();
			}
			_ => {}
		}
	}

	fn handle_main_keys(&mut self, key: event::KeyEvent) {
		match (key.code, key.modifiers) {
			(KeyCode::Char('Q'), _) => {
				let _ = Quit::new(false);
				self.should_quit = true;
			}
			(KeyCode::Char('c'), KeyModifiers::CONTROL) => {
				self.should_quit = true;
			}
			(KeyCode::Char('1'), _) => self.core.active_panel = ActivePanel::Scope,
			(KeyCode::Char('2'), _) => self.core.active_panel = ActivePanel::FileTree,
			(KeyCode::Char('3'), _) => self.core.active_panel = ActivePanel::Pending,
			(KeyCode::Char('4'), _) => self.core.active_panel = ActivePanel::ChangeList,
			(KeyCode::Tab, _)       => self.core.active_panel = ActivePanel::Detail,
			(KeyCode::Char('@'), _) => self.core.active_panel = ActivePanel::Log,

			(KeyCode::Char('S'), _) if self.core.active_panel == ActivePanel::Pending && !self.core.pending_files.is_empty() => {
				self.core.is_submit_overlay_open = true;
				self.core.submit_focus = pier_core::core::SubmitFocus::Description;
				self.core.submit_cursor = 0;
				self.core.submit_description.clear();
			}
			
			// FileTree 导航与 P4 操作
			(KeyCode::Char('j'), _) if self.core.active_panel == ActivePanel::FileTree => {
				self.core.ft_move_down();
				self.trigger_detail_update();
			}
			(KeyCode::Char('k'), _) if self.core.active_panel == ActivePanel::FileTree => {
				self.core.ft_move_up();
				self.trigger_detail_update();
			}
			(KeyCode::Char('l'), _) if self.core.active_panel == ActivePanel::FileTree => {
				self.core.ft_enter_dir();
				self.trigger_detail_update();
				self.trigger_status_update();
			}
			(KeyCode::Char('h'), _) if self.core.active_panel == ActivePanel::FileTree => {
				self.core.ft_leave_dir();
				self.trigger_detail_update();
				self.trigger_status_update();
			}
			(KeyCode::Char('c'), _) if self.core.active_panel == ActivePanel::FileTree => {
				self.core.ft_p4_edit();
			}
			(KeyCode::Char('d'), _) if self.core.active_panel == ActivePanel::FileTree => {
				self.core.ft_p4_delete();
			}
			(KeyCode::Char('r'), _) if self.core.active_panel == ActivePanel::FileTree => {
				self.core.ft_p4_revert();
			}
			(KeyCode::Char('a'), _) if self.core.active_panel == ActivePanel::FileTree => {
				self.core.ft_p4_add();
			}
			(KeyCode::Char('V'), _) if self.core.active_panel == ActivePanel::FileTree => {
				if let Some(file) = self.core.filetree.files.get(self.core.filetree.selected) {
					if file.is_dir {
						if self.core.virtual_root.as_ref() == Some(&file.path) {
							self.core.virtual_root = None;
						} else {
							self.core.virtual_root = Some(file.path.clone());
							self.core.add_to_virtual_root_history(file.path.clone());
						}
						self.core.save_config();
						// Refresh changelists for the new virtual root
						if let Ok(cls) = pier_core::changelist::fetch_changelists(&self.core.client_root, self.core.virtual_root.as_deref()) {
							self.core.changelists = cls;
							self.core.cl_cursor = 0;
							self.core.expanded_ids.clear();
						}
					}
				}
			}

			// Pending 导航与操作
			(KeyCode::Char('j'), _) if self.core.active_panel == ActivePanel::Pending => {
				self.core.pd_move_down();
			}
			(KeyCode::Char('k'), _) if self.core.active_panel == ActivePanel::Pending => {
				self.core.pd_move_up();
			}
			(KeyCode::Char('l'), _) if self.core.active_panel == ActivePanel::Pending => {
				self.core.pd_expand();
			}
			(KeyCode::Char('h'), _) if self.core.active_panel == ActivePanel::Pending => {
				self.core.pd_collapse();
			}
			(KeyCode::Char('r'), _) if self.core.active_panel == ActivePanel::Pending => {
				self.core.pd_p4_revert();
			}

			// ChangeList 导航按键
			(KeyCode::Char('j'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				self.core.cl_move_down();
			}
			(KeyCode::Char('k'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				self.core.cl_move_up();
			}
			(KeyCode::Char('l') | KeyCode::Enter, _) if self.core.active_panel == ActivePanel::ChangeList => {
				self.core.cl_expand();
			}
			(KeyCode::Char('h'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				self.core.cl_collapse();
			}
			(KeyCode::Char('f'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				if let Ok(cls) = pier_core::changelist::fetch_changelists(&self.core.client_root, self.core.virtual_root.as_deref()) {
					self.core.changelists = cls;
				}
				self.last_cl_refresh = std::time::Instant::now();
			}
			
			(KeyCode::Enter, _) if self.core.active_panel == ActivePanel::Scope => {
				self.core.is_info_overlay_open = true;
				self.core.info_focus = pier_core::core::InfoFocus::Roots;
				self.core.update_p4_info_details();
			}
			(KeyCode::Char('F'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				if let Some(target) = self.core.get_cl_target_at(self.core.cl_cursor) {
					if let pier_core::core::ClTarget::File(_, depot_path) = target {
						if let Some(local_path) = self.core.cl_get_local_path(&depot_path) {
							if self.core.jump_to_file(&local_path) {
								self.trigger_detail_update();
								self.trigger_status_update();
							}
						}
					}
				}
			}
			(KeyCode::Char('s'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				if let Ok(cls) = pier_core::changelist::fetch_changelists(&self.core.client_root, self.core.virtual_root.as_deref()) {
					self.core.changelists = cls;
				}
				self.start_sync(None);
				self.last_cl_refresh = std::time::Instant::now();
			}
			(KeyCode::Char('g'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				if let Some(target) = self.core.get_cl_target_at(self.core.cl_cursor) {
					if let pier_core::core::ClTarget::Id(id) = target {
						self.start_sync(Some(id));
					}
				}
			}

			// Detail 导航与复制按键
			(KeyCode::Char('j'), _) if self.core.active_panel == ActivePanel::Detail => {
				self.core.dt_move_down();
			}
			(KeyCode::Char('k'), _) if self.core.active_panel == ActivePanel::Detail => {
				self.core.dt_move_up();
			}
			(KeyCode::Char('Y'), _) if self.core.active_panel == ActivePanel::Detail => {
				self.core.dt_copy_selected();
			}

			// Log 导航按键
			(KeyCode::Char('j'), _) if self.core.active_panel == ActivePanel::Log => {
				self.core.log_move_down();
			}
			(KeyCode::Char('k'), _) if self.core.active_panel == ActivePanel::Log => {
				self.core.log_move_up();
			}
			_ => {}
		}
	}

	fn trigger_detail_update(&mut self) {
		if let Some(file) = self.core.filetree.files.get(self.core.filetree.selected) {
			if let Some(cached) = pier_core::detail::load_from_cache(&file.path) {
				self.core.current_detail = Some(cached);
				self.core.detail_error = None;
			}
			let _ = self.detail_tx.send(file.path.clone());
		}
	}

	fn trigger_status_update(&mut self) {
		let mut paths = Vec::new();
		for file in self.core.filetree.files.iter().chain(self.core.filetree.parent_files.iter()) {
			if !file.is_dir {
				paths.push(file.path.clone());
			}
		}
		let _ = self.status_tx.send(paths);
	}

	fn start_sync(&mut self, cl_id: Option<String>) {
		if self.core.is_syncing { return; }
		self.core.is_syncing = true;
		self.core.sync_progress = 0.0;
		self.core.sync_files.clear();
		self.core.sync_total = 0;
		self.core.sync_current = 0;
		self.core.sync_total_bytes = 0;
		self.core.sync_synced_bytes = 0;

		let tx = self.sync_tx.clone();
		let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
		self.sync_cancel_tx = Some(cancel_tx);

		let virtual_root = self.core.virtual_root.clone();
		let handle = tokio::spawn(async move {
			// 1. Dry run to get files with ztag
			let mut cmd = tokio::process::Command::new("p4");
			cmd.arg("-ztag").arg("sync").arg("-n");
			if let Some(id) = &cl_id {
				cmd.arg(format!("@{}", id));
			}
			
			if let Some(ref vr) = virtual_root {
				let vr_str = vr.to_string_lossy();
				cmd.arg(format!("{}/...", vr_str));
			}
			
			let output = cmd.output().await;
			let files = if let Ok(out) = output {
				parse_ztag_sync(&String::from_utf8_lossy(&out.stdout))
			} else {
				vec![]
			};

			if files.is_empty() {
				let _ = tx.send(SyncEvent::End);
				return;
			}

			// Local paths for progress tracking
			let sync_info: Vec<(String, u64)> = files.iter().map(|f| (f.local_path.clone(), f.size)).collect();
			
			let _ = tx.send(SyncEvent::Start(files));

			// 2. Actual sync
			let mut cmd = tokio::process::Command::new("p4");
			cmd.arg("sync");
			if let Some(id) = cl_id {
				cmd.arg(format!("@{}", id));
			}

			if let Some(ref vr) = virtual_root {
				let vr_str = vr.to_string_lossy();
				cmd.arg(format!("{}/...", vr_str));
			}

			cmd.stdout(std::process::Stdio::piped());
			cmd.stderr(std::process::Stdio::piped());
			
			let mut child = match cmd.spawn() {
				Ok(c) => c,
				Err(e) => {
					let _ = tx.send(SyncEvent::Error(e.to_string()));
					return;
				}
			};

			let stdout = child.stdout.take().unwrap();
			let mut reader = tokio::io::BufReader::new(stdout);
			
			let (done_tx, mut done_rx) = tokio::sync::oneshot::channel::<()>();
			let tx_progress = tx.clone();
			
			// Progress monitor task
			tokio::spawn(async move {
				let mut interval = interval(Duration::from_millis(100));
				loop {
					tokio::select! {
						_ = &mut done_rx => break,
						_ = interval.tick() => {
							let mut progress_vec = Vec::with_capacity(sync_info.len());
							for (path, expected_size) in &sync_info {
								let synced = if let Ok(metadata) = fs::metadata(path) {
									metadata.len().min(*expected_size)
								} else {
									0
								};
								progress_vec.push(synced);
							}
							let _ = tx_progress.send(SyncEvent::ByteProgress(progress_vec));
						}
					}
				}
			});

			loop {
				let mut line = String::new();
				tokio::select! {
					_ = &mut cancel_rx => {
						let _ = child.kill().await;
						let _ = done_tx.send(());
						return;
					}
					res = reader.read_line(&mut line) => {
						match res {
							Ok(0) => break,
							Ok(_) => {
								let trimmed = line.trim();
								if !trimmed.is_empty() {
									let _ = tx.send(SyncEvent::FileDone(trimmed.to_string()));
								}
							}
							Err(_) => break,
						}
					}
				}
			}

			let _ = child.wait().await;
			let _ = done_tx.send(());
			let _ = tx.send(SyncEvent::End);
		});
		self.sync_handle = Some(handle);
	}
}

fn parse_ztag_sync(output: &str) -> Vec<pier_core::core::SyncFileInfo> {
	let mut files = Vec::new();
	let mut depot_path = String::new();
	let mut local_path = String::new();
	let mut size = 0;

	for line in output.lines() {
		if let Some(rest) = line.strip_prefix("... depotFile ") {
			if !depot_path.is_empty() {
				files.push(pier_core::core::SyncFileInfo {
					depot_path: std::mem::take(&mut depot_path),
					local_path: std::mem::take(&mut local_path),
					size,
					synced: 0,
					original_index: files.len(),
				});
				size = 0;
			}
			depot_path = rest.to_string();
		} else if let Some(rest) = line.strip_prefix("... clientFile ") {
			local_path = rest.to_string();
		} else if let Some(rest) = line.strip_prefix("... fileSize ") {
			size = rest.parse().unwrap_or(0);
		}
	}

	if !depot_path.is_empty() {
		files.push(pier_core::core::SyncFileInfo {
			depot_path,
			local_path,
			size,
			synced: 0,
			original_index: files.len(),
		});
	}

	files
}
