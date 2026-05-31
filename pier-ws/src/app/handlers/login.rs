use crossterm::event::KeyCode;
use crate::app::App;

pub(crate) fn handle(app: &mut App, key: crossterm::event::KeyEvent) {
	match (key.code, key.modifiers) {
		(KeyCode::Esc, _) => app.should_quit = true,
		(KeyCode::Enter, _) => {
			app.core.p4_login();
		}
		(KeyCode::Char(c), _) => {
			app.core.login_password.push(c);
		}
		(KeyCode::Backspace, _) => {
			app.core.login_password.pop();
		}
		_ => {}
	}
}
