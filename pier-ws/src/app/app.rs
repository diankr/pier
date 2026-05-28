use std::{
	io::{self, Stdout},
	time::Duration,
	path::PathBuf,
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
use tokio::time::sleep;

use super::commands::Quit;

pub type Term = Terminal<CrosstermBackend<Stdout>>;

pub(crate) struct App {
	pub(crate) core: Core,
	pub(crate) term: Term,
	pub(crate) state: UiState,
	pub(crate) should_quit: bool,
	detail_tx: tokio::sync::mpsc::UnboundedSender<PathBuf>,
	detail_rx: tokio::sync::mpsc::UnboundedReceiver<Result<pier_core::detail::FileDetail, String>>,
	status_tx: tokio::sync::mpsc::UnboundedSender<Vec<PathBuf>>,
	status_rx: tokio::sync::mpsc::UnboundedReceiver<std::collections::HashMap<PathBuf, pier_core::filetree::FileP4Status>>,
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

		Ok(Self {
			core,
			term,
			state: UiState::new(),
			should_quit: false,
			detail_tx: request_tx,
			detail_rx: result_rx,
			status_tx: status_request_tx,
			status_rx: status_result_rx,
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

			if self.last_cl_refresh.elapsed() >= Duration::from_secs(60) {
				if let Ok(cls) = pier_core::changelist::fetch_changelists(&self.core.client_root) {
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
						if self.core.is_login_overlay_open {
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
				if let Some(target) = self.core.get_cl_target_at(self.core.cl_cursor) {
					match target {
						pier_core::core::ClTarget::Id(_) => self.core.cl_expand(),
						pier_core::core::ClTarget::File(_, depot_path) => {
							if let Some(local_path) = self.core.cl_get_local_path(&depot_path) {
								if self.core.jump_to_file(&local_path) {
									self.trigger_detail_update();
									self.trigger_status_update();
								}
							}
						}
					}
				}
			}
			(KeyCode::Char('h'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				self.core.cl_collapse();
			}
			(KeyCode::Char('f'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				if let Ok(cls) = pier_core::changelist::fetch_changelists(&self.core.client_root) {
					self.core.changelists = cls;
				}
				self.last_cl_refresh = std::time::Instant::now();
			}
			(KeyCode::Char('s'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				if let Ok(cls) = pier_core::changelist::fetch_changelists(&self.core.client_root) {
					self.core.changelists = cls;
				}
				self.core.p4_sync_latest();
				self.last_cl_refresh = std::time::Instant::now();
			}
			(KeyCode::Char('g'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				if let Some(target) = self.core.get_cl_target_at(self.core.cl_cursor) {
					if let pier_core::core::ClTarget::Id(id) = target {
						self.core.p4_sync_cl(&id);
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
}
