use crossterm::event::{KeyCode, KeyModifiers};
use pier_core::core::ActivePanel;
use crate::app::App;

pub(crate) fn handle(app: &mut App, key: crossterm::event::KeyEvent) {
	match (key.code, key.modifiers) {
		(KeyCode::Char('Q'), _) => {
			app.should_quit = true;
		}
		(KeyCode::Char('c'), KeyModifiers::CONTROL) => {
			app.should_quit = true;
		}
		(KeyCode::Char('1'), _) => app.core.active_panel = ActivePanel::Scope,
		(KeyCode::Char('2'), _) => app.core.active_panel = ActivePanel::FileTree,
		(KeyCode::Char('3'), _) => app.core.active_panel = ActivePanel::Pending,
		(KeyCode::Char('4'), _) => app.core.active_panel = ActivePanel::ChangeList,
		(KeyCode::Tab, _)       => app.core.active_panel = ActivePanel::Detail,
		(KeyCode::Char('@'), _) => app.core.active_panel = ActivePanel::Log,

		(KeyCode::Char('S'), _) if app.core.active_panel == ActivePanel::Pending && !app.core.pending_files.is_empty() => {
			app.core.is_submit_overlay_open = true;
			app.core.submit_focus = pier_core::core::SubmitFocus::Description;
			app.core.submit_cursor = 0;
			app.core.submit_description.clear();
		}
		
		// FileTree 导航与 P4 操作
		(KeyCode::Char('j'), _) if app.core.active_panel == ActivePanel::FileTree => {
			app.core.ft_move_down();
			app.trigger_detail_update();
		}
		(KeyCode::Char('k'), _) if app.core.active_panel == ActivePanel::FileTree => {
			app.core.ft_move_up();
			app.trigger_detail_update();
		}
		(KeyCode::Char('l'), _) if app.core.active_panel == ActivePanel::FileTree => {
			app.core.ft_enter_dir();
			app.trigger_detail_update();
			app.trigger_status_update();
		}
		(KeyCode::Char('h'), _) if app.core.active_panel == ActivePanel::FileTree => {
			app.core.ft_leave_dir();
			app.trigger_detail_update();
			app.trigger_status_update();
		}
		(KeyCode::Char('c'), _) if app.core.active_panel == ActivePanel::FileTree => {
			app.core.ft_p4_edit();
		}
		(KeyCode::Char('d'), _) if app.core.active_panel == ActivePanel::FileTree => {
			app.core.ft_p4_delete();
		}
		(KeyCode::Char('r'), _) if app.core.active_panel == ActivePanel::FileTree => {
			app.core.ft_p4_revert();
		}
		(KeyCode::Char('a'), _) if app.core.active_panel == ActivePanel::FileTree => {
			app.core.ft_p4_add();
		}
		(KeyCode::Char('V'), _) if app.core.active_panel == ActivePanel::FileTree => {
			if let Some(file) = app.core.filetree.files.get(app.core.filetree.selected) {
				if file.is_dir {
					if app.core.virtual_root.as_ref() == Some(&file.path) {
						app.core.virtual_root = None;
					} else {
						app.core.virtual_root = Some(file.path.clone());
						app.core.add_to_virtual_root_history(file.path.clone());
					}
					app.core.save_config();
					// Refresh changelists for the new virtual root
					if let Ok(cls) = pier_core::changelist::fetch_changelists(&app.core.client_root, app.core.virtual_root.as_deref()) {
						app.core.changelists = cls;
						app.core.cl_cursor = 0;
						app.core.expanded_ids.clear();
					}
				}
			}
		}

		// Pending 导航与操作
		(KeyCode::Char('j'), _) if app.core.active_panel == ActivePanel::Pending => {
			app.core.pd_move_down();
		}
		(KeyCode::Char('k'), _) if app.core.active_panel == ActivePanel::Pending => {
			app.core.pd_move_up();
		}
		(KeyCode::Char('l'), _) if app.core.active_panel == ActivePanel::Pending => {
			app.core.pd_expand();
		}
		(KeyCode::Char('h'), _) if app.core.active_panel == ActivePanel::Pending => {
			app.core.pd_collapse();
		}
		(KeyCode::Char('r'), _) if app.core.active_panel == ActivePanel::Pending => {
			app.core.pd_p4_revert();
		}

		// ChangeList 导航按键
		(KeyCode::Char('j'), _) if app.core.active_panel == ActivePanel::ChangeList => {
			app.core.cl_move_down();
		}
		(KeyCode::Char('k'), _) if app.core.active_panel == ActivePanel::ChangeList => {
			app.core.cl_move_up();
		}
		(KeyCode::Char('l') | KeyCode::Enter, _) if app.core.active_panel == ActivePanel::ChangeList => {
			app.core.cl_expand();
		}
		(KeyCode::Char('h'), _) if app.core.active_panel == ActivePanel::ChangeList => {
			app.core.cl_collapse();
		}
		(KeyCode::Char('f'), _) if app.core.active_panel == ActivePanel::ChangeList => {
			if let Ok(cls) = pier_core::changelist::fetch_changelists(&app.core.client_root, app.core.virtual_root.as_deref()) {
				app.core.changelists = cls;
			}
			app.last_cl_refresh = std::time::Instant::now();
		}
		
		(KeyCode::Enter, _) if app.core.active_panel == ActivePanel::Scope => {
			app.core.is_info_overlay_open = true;
			app.core.info_focus = pier_core::core::InfoFocus::Roots;
			app.core.update_p4_info_details();
		}
		(KeyCode::Char('F'), _) if app.core.active_panel == ActivePanel::ChangeList => {
			if let Some(target) = app.core.get_cl_target_at(app.core.cl_cursor) {
				if let pier_core::core::ClTarget::File(_, depot_path) = target {
					if let Some(local_path) = app.core.cl_get_local_path(&depot_path) {
						if app.core.jump_to_file(&local_path) {
							app.trigger_detail_update();
							app.trigger_status_update();
						}
					}
				}
			}
		}
		(KeyCode::Char('s'), _) if app.core.active_panel == ActivePanel::ChangeList => {
			if let Ok(cls) = pier_core::changelist::fetch_changelists(&app.core.client_root, app.core.virtual_root.as_deref()) {
				app.core.changelists = cls;
			}
			app.start_sync(None);
			app.last_cl_refresh = std::time::Instant::now();
		}
		(KeyCode::Char('g'), _) if app.core.active_panel == ActivePanel::ChangeList => {
			if let Some(target) = app.core.get_cl_target_at(app.core.cl_cursor) {
				if let pier_core::core::ClTarget::Id(id) = target {
					app.start_sync(Some(id));
				}
			}
		}

		// Detail 导航与复制按键
		(KeyCode::Char('j'), _) if app.core.active_panel == ActivePanel::Detail => {
			app.core.dt_move_down();
		}
		(KeyCode::Char('k'), _) if app.core.active_panel == ActivePanel::Detail => {
			app.core.dt_move_up();
		}
		(KeyCode::Char('Y'), _) if app.core.active_panel == ActivePanel::Detail => {
			app.core.dt_copy_selected();
		}

		// Log 导航按键
		(KeyCode::Char('j'), _) if app.core.active_panel == ActivePanel::Log => {
			app.core.log_move_down();
		}
		(KeyCode::Char('k'), _) if app.core.active_panel == ActivePanel::Log => {
			app.core.log_move_up();
		}
		_ => {}
	}
}
