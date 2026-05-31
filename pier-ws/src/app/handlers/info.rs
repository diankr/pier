use crossterm::event::KeyCode;
use crate::app::App;

pub(crate) fn handle(app: &mut App, key: crossterm::event::KeyEvent) {
	match (key.code, key.modifiers) {
		(KeyCode::Esc, _) => app.core.is_info_overlay_open = false,
		(KeyCode::Tab, _) => {
			app.core.info_focus = match app.core.info_focus {
				pier_core::core::InfoFocus::Roots => pier_core::core::InfoFocus::Details,
				pier_core::core::InfoFocus::Details => pier_core::core::InfoFocus::Roots,
			};
		}
		(KeyCode::Char('j'), _) => {
			match app.core.info_focus {
				pier_core::core::InfoFocus::Roots => {
					let max = if app.core.is_roots_expanded { app.core.virtual_root_history.len() } else { 0 };
					if app.core.info_roots_cursor < max {
						app.core.info_roots_cursor += 1;
					}
				}
				pier_core::core::InfoFocus::Details => {
					if app.core.info_details_cursor < app.core.info_details.len().saturating_sub(1) {
						app.core.info_details_cursor += 1;
					}
				}
			}
		}
		(KeyCode::Char('k'), _) => {
			match app.core.info_focus {
				pier_core::core::InfoFocus::Roots => {
					if app.core.info_roots_cursor > 0 {
						app.core.info_roots_cursor -= 1;
					}
				}
				pier_core::core::InfoFocus::Details => {
					if app.core.info_details_cursor > 0 {
						app.core.info_details_cursor -= 1;
					}
				}
			}
		}
		(KeyCode::Char('l'), _) if app.core.info_focus == pier_core::core::InfoFocus::Roots => {
			app.core.is_roots_expanded = true;
		}
		(KeyCode::Char('h'), _) if app.core.info_focus == pier_core::core::InfoFocus::Roots => {
			app.core.is_roots_expanded = false;
			app.core.info_roots_cursor = 0;
		}
		(KeyCode::Enter, _) if app.core.info_focus == pier_core::core::InfoFocus::Roots => {
			let target_path = if app.core.info_roots_cursor == 0 {
				Some(app.core.client_root.clone())
			} else if let Some(vr) = app.core.virtual_root_history.get(app.core.info_roots_cursor - 1) {
				Some(vr.clone())
			} else {
				None
			};

			if let Some(path) = target_path {
				if app.core.info_roots_cursor == 0 {
					app.core.virtual_root = None;
				} else {
					app.core.virtual_root = Some(path.clone());
				}
				app.core.save_config();
				app.core.enter_path(&path);
				app.core.is_info_overlay_open = false;
				// Refresh changelists for the new virtual root
				if let Ok(cls) = pier_core::changelist::fetch_changelists(&app.core.client_root, app.core.virtual_root.as_deref()) {
					app.core.changelists = cls;
					app.core.cl_cursor = 0;
					app.core.expanded_ids.clear();
				}
			}
		}
		(KeyCode::Char('y'), _) if app.core.info_focus == pier_core::core::InfoFocus::Details => {
			if let Some((_, v)) = app.core.info_details.get(app.core.info_details_cursor) {
				use arboard::Clipboard;
				if let Ok(mut clipboard) = Clipboard::new() {
					let _ = clipboard.set_text(v.clone());
				}
			}
		}
		_ => {}
	}
}
