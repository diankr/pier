use std::{
	io::{self, Stdout},
	time::Duration,
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
}

impl App {
	fn new() -> Result<Self> {
		let backend = CrosstermBackend::new(io::stdout());
		let term = Terminal::new(backend)?;
		Ok(Self {
			core: Core::new(),
			term,
			state: UiState::new(),
			should_quit: false,
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
		loop {
			if self.should_quit {
				break;
			}

			self.term.draw(|f| {
				let area = f.area();
				render_root(f, area, &self.state, &self.core);
			})?;

			if event::poll(Duration::from_millis(10))? {
				if let Event::Key(key) = event::read()? {
					if key.kind == KeyEventKind::Press {
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
							(KeyCode::Char('4'), _) => self.core.active_panel = ActivePanel::Detail,
							(KeyCode::Char('5'), _) => self.core.active_panel = ActivePanel::Log,
							
							// FileTree 导航按键
							(KeyCode::Char('j'), _) if self.core.active_panel == ActivePanel::FileTree => {
								self.core.filetree.move_down();
							}
							(KeyCode::Char('k'), _) if self.core.active_panel == ActivePanel::FileTree => {
								self.core.filetree.move_up();
							}
							(KeyCode::Char('l'), _) if self.core.active_panel == ActivePanel::FileTree => {
								self.core.filetree.enter_dir();
							}
							(KeyCode::Char('h'), _) if self.core.active_panel == ActivePanel::FileTree => {
								self.core.filetree.leave_dir();
							}
							_ => {}
						}
					}
				}
			}

			sleep(Duration::from_millis(10)).await;
		}
		Ok(())
	}
}
