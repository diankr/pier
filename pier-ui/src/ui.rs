use pier_core::core::{ActivePanel, Core};
use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	Frame,
};

use crate::components::*;

pub struct UiState {
	pub is_scope_expanded: bool,
}

impl UiState {
	pub fn new() -> Self {
		Self {
			is_scope_expanded: false,
		}
	}
}

pub fn render_root(f: &mut Frame, area: Rect, state: &UiState, core: &Core) {
	let root_chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Min(1),
			Constraint::Length(1),
		])
		.split(area);

	let main_rect = root_chunks[0];
	let footer_area = root_chunks[1];

	let main_chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
		.split(main_rect);

	let left_area = main_chunks[0];
	let right_area = main_chunks[1];

	let is_pd_active = core.active_panel == ActivePanel::Pending;
	let is_cl_active = core.active_panel == ActivePanel::ChangeList;
	let is_log_active = core.active_panel == ActivePanel::Log;

	let scope_constraint = if state.is_scope_expanded {
		Constraint::Percentage(50)
	} else {
		Constraint::Length(3)
	};

	let pending_constraint = if is_pd_active {
		Constraint::Length(20)
	} else {
		Constraint::Length(10)
	};

	let left_chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			scope_constraint,     // Domain
			Constraint::Min(10),  // FileTree
			pending_constraint,   // Pending
		])
		.split(left_area);

	let scope_area = left_chunks[0];
	let filetree_area = left_chunks[1];
	let pending_area = left_chunks[2];

	// Right Area Logic
	let (changelist_area, detail_area, log_area) = if is_log_active {
		(Rect::default(), Rect::default(), right_area)
	} else {
		let log_height = 10;
		let right_parts = Layout::default()
			.direction(Direction::Vertical)
			.constraints([
				Constraint::Min(10), // CL + Detail
				Constraint::Length(log_height), // Log
			])
			.split(right_area);
		
		let cl_constraint = if is_cl_active {
			Constraint::Percentage(66)
		} else {
			Constraint::Percentage(50)
		};
		let dt_constraint = if is_cl_active {
			Constraint::Percentage(34)
		} else {
			Constraint::Percentage(50)
		};

		let top_parts = Layout::default()
			.direction(Direction::Vertical)
			.constraints([cl_constraint, dt_constraint])
			.split(right_parts[0]);
		
		(top_parts[0], top_parts[1], right_parts[1])
	};

	// Domain
	Domain::render(f, scope_area, core);
	
	// FileTree
	FileTree::render(f, filetree_area, core);

	// Pending
	Pending::render(f, pending_area, core);

	// Right Panels
	if !is_log_active {
		ChangeList::render(f, changelist_area, core);
		Detail::render(f, detail_area, core);
	}

	Log::render(f, log_area, core);

	// Footer
	Footer::render(f, footer_area, core);
	
	// Overlays
	if core.is_info_overlay_open {
		Info::render(f, area, core);
	}

	if core.is_submit_overlay_open {
		Submit::render(f, area, core);
	}

	if core.is_login_overlay_open {
		Login::render(f, area, core);
	}

	if core.is_syncing {
		Sync::render(f, area, core);
	}
}
