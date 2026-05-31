use crossterm::event::KeyCode;
use crate::app::App;

pub(crate) fn handle(app: &mut App, key: crossterm::event::KeyEvent) {
	match (key.code, key.modifiers) {
		(KeyCode::Esc, _) => app.core.is_submit_overlay_open = false,
		(KeyCode::Tab, _) => {
			app.core.submit_focus = match app.core.submit_focus {
				pier_core::core::SubmitFocus::Description => pier_core::core::SubmitFocus::FileList,
				pier_core::core::SubmitFocus::FileList => pier_core::core::SubmitFocus::Description,
			};
		}
		(KeyCode::Char('j'), _) if app.core.submit_focus == pier_core::core::SubmitFocus::FileList => {
			if app.core.submit_cursor < app.core.pending_files.len().saturating_sub(1) {
				app.core.submit_cursor += 1;
			}
		}
		(KeyCode::Char('k'), _) if app.core.submit_focus == pier_core::core::SubmitFocus::FileList => {
			if app.core.submit_cursor > 0 {
				app.core.submit_cursor -= 1;
			}
		}
		(KeyCode::Enter, _) => {
			app.core.p4_submit();
		}

		(KeyCode::Char(c), _) if app.core.submit_focus == pier_core::core::SubmitFocus::Description => {
			app.core.submit_description.push(c);
		}
		(KeyCode::Backspace, _) if app.core.submit_focus == pier_core::core::SubmitFocus::Description => {
			app.core.submit_description.pop();
		}
		_ => {}
	}
}
