use std::{io::Stdout, sync::atomic::Ordering, time::{Duration, Instant}};

use anyhow::Result;
use pier_ui::ui::render_root;
use tokio::{select, time::sleep};
use pier_core::Core;
// use pier_ui::Core;
// use pier_macro;

// use crate::{Dispatcher, Signals, Term};
use ratatui::{Terminal, backend::{self, CrosstermBackend}};
pub type Term = Terminal<CrosstermBackend<Stdout>>;

pub(crate) struct App {
	pub(crate) core: Core,
	pub(crate) term: Option<Term>,
	// pub(crate) signals: Signals,
}

impl App {
	pub(crate) async fn serve() -> Result<()> {
		// let term = Term::start();
		let backend = CrosstermBackend::new(std::io::stdout());
		// let terminal = Terminal::new(backend)?;

		// let mut app = App {
		// 	core:Core::new(),
		// 	term: Some(terminal),
		// };

		let mut term = Terminal::new(backend)?;
		let mut state = pier_ui::ui::UiState::new();

		let temp: bool = true;
		let timeout = Duration::from_millis(10);

		loop {
			if temp {
				term.draw(|f| {
					let area = f.area();
					render_root(f, area, &state);
				})?;
				sleep(Duration::from_millis(10)).await;
			} else {
				break;
			}
		}
		Ok(())
	}
}
