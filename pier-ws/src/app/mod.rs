use std::{
	io::{self, Stdout},
	time::Duration,
	path::PathBuf,
};

use anyhow::Result;
use crossterm::{
	event::{self, Event, KeyCode, KeyEventKind},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pier_core::core::Core;
use pier_ui::ui::{render_root, UiState};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::time::sleep;

pub(crate) mod handlers;
pub(crate) mod worker;
pub(crate) mod sync;
pub(crate) mod dispatcher;

use sync::SyncEvent;
use dispatcher::Dispatcher;

pub type Term = Terminal<CrosstermBackend<Stdout>>;

pub(crate) struct App {
	pub(crate) core: Core,
	pub(crate) term: Term,
	pub(crate) state: UiState,
	pub(crate) should_quit: bool,
	pub(crate) detail_tx: tokio::sync::mpsc::UnboundedSender<PathBuf>,
	pub(crate) detail_rx: tokio::sync::mpsc::UnboundedReceiver<Result<pier_core::detail::FileDetail, String>>,
	pub(crate) status_tx: tokio::sync::mpsc::UnboundedSender<Vec<PathBuf>>,
	pub(crate) status_rx: tokio::sync::mpsc::UnboundedReceiver<std::collections::HashMap<PathBuf, pier_core::filetree::FileP4Status>>,
	
	pub(crate) sync_tx: tokio::sync::mpsc::UnboundedSender<SyncEvent>,
	pub(crate) sync_rx: tokio::sync::mpsc::UnboundedReceiver<SyncEvent>,
	pub(crate) sync_handle: Option<tokio::task::JoinHandle<()>>,
	pub(crate) sync_cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,

	pub(crate) last_cl_refresh: std::time::Instant,
}

impl App {
	fn new() -> Result<Self> {
		let core = Core::new().map_err(|e| anyhow::anyhow!(e))?;
		let backend = CrosstermBackend::new(io::stdout());
		let term = Terminal::new(backend)?;

		let (detail_request_tx, detail_request_rx) = tokio::sync::mpsc::unbounded_channel::<PathBuf>();
		let (detail_result_tx, detail_result_rx) = tokio::sync::mpsc::unbounded_channel::<Result<pier_core::detail::FileDetail, String>>();
		worker::spawn_detail_worker(detail_request_rx, detail_result_tx);

		let (status_request_tx, status_request_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<PathBuf>>();
		let (status_result_tx, status_result_rx) = tokio::sync::mpsc::unbounded_channel::<std::collections::HashMap<PathBuf, pier_core::filetree::FileP4Status>>();
		worker::spawn_status_worker(status_request_rx, status_result_tx);

		let (sync_tx, sync_rx) = tokio::sync::mpsc::unbounded_channel::<SyncEvent>();

		Ok(Self {
			core,
			term,
			state: UiState::new(),
			should_quit: false,
			detail_tx: detail_request_tx,
			detail_rx: detail_result_rx,
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

			Dispatcher::new(self).process_events()?;

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
						self.handle_key_event(key);
					}
				}
			}
			sleep(Duration::from_millis(10)).await;
		}
		Ok(())
	}

	fn handle_key_event(&mut self, key: event::KeyEvent) {
		if self.core.is_syncing {
			if key.code == KeyCode::Esc {
				if let Some(cancel) = self.sync_cancel_tx.take() {
					let _ = cancel.send(());
				}
				self.core.is_syncing = false;
			}
		} else if self.core.is_info_overlay_open {
			handlers::info::handle(self, key);
		} else if self.core.is_login_overlay_open {
			handlers::login::handle(self, key);
		} else if self.core.is_submit_overlay_open {
			handlers::submit::handle(self, key);
		} else {
			handlers::main::handle(self, key);
		}
	}

	pub(crate) fn trigger_detail_update(&mut self) {
		if let Some(file) = self.core.filetree.files.get(self.core.filetree.selected) {
			if let Some(cached) = pier_core::detail::load_from_cache(&file.path) {
				self.core.current_detail = Some(cached);
				self.core.detail_error = None;
			}
			let _ = self.detail_tx.send(file.path.clone());
		}
	}

	pub(crate) fn trigger_status_update(&mut self) {
		let mut paths = Vec::new();
		for file in self.core.filetree.files.iter().chain(self.core.filetree.parent_files.iter()) {
			if !file.is_dir {
				paths.push(file.path.clone());
			}
		}
		let _ = self.status_tx.send(paths);
	}

	pub(crate) fn start_sync(&mut self, cl_id: Option<String>) {
		if self.core.is_syncing { return; }
		self.core.is_syncing = true;
		self.core.sync_progress = 0.0;
		self.core.sync_files.clear();
		self.core.sync_total = 0;
		self.core.sync_current = 0;
		self.core.sync_total_bytes = 0;
		self.core.sync_synced_bytes = 0;

		let tx = self.sync_tx.clone();
		let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
		self.sync_cancel_tx = Some(cancel_tx);

		let virtual_root = self.core.virtual_root.clone();
		self.sync_handle = Some(sync::spawn_sync_task(cl_id, virtual_root, tx, cancel_rx));
	}
}
