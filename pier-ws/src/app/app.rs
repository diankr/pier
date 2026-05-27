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

		Ok(Self {
			core,
			term,
			state: UiState::new(),
			should_quit: false,
			detail_tx: request_tx,
			detail_rx: result_rx,
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

			self.term.draw(|f| {
				let area = f.area();
				render_root(f, area, &self.state, &self.core);
			})?;

			if event::poll(Duration::from_millis(10))? {
				if let Event::Key(key) = event::read()? {
					if key.kind == KeyEventKind::Press {
						if self.core.is_submit_overlay_open {
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
			(KeyCode::Enter, _) if self.core.submit_focus == pier_core::core::SubmitFocus::FileList => {
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
			}
			(KeyCode::Char('h'), _) if self.core.active_panel == ActivePanel::FileTree => {
				self.core.ft_leave_dir();
				self.trigger_detail_update();
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
				self.core.cl_expand();
			}
			(KeyCode::Char('h'), _) if self.core.active_panel == ActivePanel::ChangeList => {
				self.core.cl_collapse();
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
}
