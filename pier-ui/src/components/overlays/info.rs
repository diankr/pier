use pier_config::theme;
use pier_core::core::{Core, InfoFocus};
use ratatui::{
	layout::{Constraint, Layout, Rect},
	style::{Modifier, Style},
	widgets::{Block, Borders, List, ListItem},
	text::{Line, Span},
	Frame,
};

pub(crate) struct Info;

impl Info {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let info_count = core.info_details.len();
		// Fixed height for roots (6) + dynamic height for info (min 6, max 17)
		let info_height_clamped = (info_count + 2).clamp(6, 17) as u16;
		let total_height = 6 + info_height_clamped;
		let total_width = (area.width * 70 / 100).max(40);
		
		let overlay_area = Rect {
			x: area.x + (area.width.saturating_sub(total_width)) / 2,
			y: area.y + (area.height.saturating_sub(total_height)) / 2,
			width: total_width,
			height: total_height.min(area.height),
		};

		f.render_widget(ratatui::widgets::Clear, overlay_area);

		let chunks = Layout::default()
			.direction(ratatui::layout::Direction::Vertical)
			.constraints([
				Constraint::Length(6),                  // Roots (fixed 4 lines + 2 borders)
				Constraint::Length(info_height_clamped), // P4 Info (max 15 lines + 2 borders)
			])
			.split(overlay_area);

		// Upper Block: Roots
		let roots_block = Block::default()
			.title(Line::from("─Roots ").alignment(ratatui::layout::Alignment::Center))
			.borders(Borders::ALL)
			.border_set(ratatui::symbols::border::ROUNDED)
			.border_style(Style::default().fg(if core.info_focus == InfoFocus::Roots { theme().component.active_pane_border } else { theme().component.pane_border }));

		let mut roots_items = Vec::new();
		
		// Client Root
		let is_cr_active = core.virtual_root.is_none();
		let cr_icon = &theme().icon.client_root;
		let cr_color = if is_cr_active { theme().selection.cursor_bg } else { theme().component.default_text };
		let cr_check = if is_cr_active { format!(" {}", theme().icon.check) } else { "".to_string() };
		let expand_icon = if core.is_roots_expanded { "v" } else { ">" };
		
		let cr_line = Line::from(vec![
			Span::raw(format!(" {} ", expand_icon)),
			Span::styled(format!("{} ", cr_icon), Style::default().fg(cr_color)),
			Span::styled("client root 1", Style::default().fg(cr_color)),
			Span::styled(cr_check, Style::default().fg(cr_color)),
			// Right shift path by 2 chars (total 3 spaces)
			Span::raw(format!("   {:>width$}", core.client_root.to_string_lossy(), width = (chunks[0].width as usize).saturating_sub(27))),
		]);
		roots_items.push(ListItem::new(cr_line));

		// Virtual Roots
		if core.is_roots_expanded {
			for (i, vr) in core.virtual_root_history.iter().enumerate() {
				let is_vr_active = core.virtual_root.as_ref() == Some(vr);
				let vr_color = if is_vr_active { theme().selection.cursor_bg } else { theme().component.default_text };
				let vr_check = if is_vr_active { format!(" {}", theme().icon.check) } else { "".to_string() };
				let vr_icon = &theme().icon.virtual_root;
				
				let vr_line = Line::from(vec![
					Span::raw("    "), // Extra indentation
					Span::styled(format!("{} ", vr_icon), Style::default().fg(vr_color)),
					Span::styled(format!("virtual root {}", i + 1), Style::default().fg(vr_color)),
					Span::styled(vr_check, Style::default().fg(vr_color)),
					// Right shift path by 3 chars (total 4 spaces)
					Span::raw(format!("    {:>width$}", vr.to_string_lossy(), width = (chunks[0].width as usize).saturating_sub(33))),
				]);
				roots_items.push(ListItem::new(vr_line));
			}
		}

		// Ensure fixed height of 4 lines
		while roots_items.len() < 4 {
			roots_items.push(ListItem::new(""));
		}

		let roots_highlight_style = if core.info_focus == InfoFocus::Roots {
			Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
		} else {
			Style::default().add_modifier(Modifier::UNDERLINED)
		};

		let roots_list = List::new(roots_items)
			.block(roots_block)
			.highlight_style(roots_highlight_style)
			.style(Style::default().fg(theme().component.default_text));
		
		let mut roots_state = ratatui::widgets::ListState::default();
		roots_state.select(Some(core.info_roots_cursor));
		f.render_stateful_widget(roots_list, chunks[0], &mut roots_state);

		// Lower Block: P4 Info
		let info_block = Block::default()
			.title(Line::from("─p4 info (tab to toggle view) ").alignment(ratatui::layout::Alignment::Center))
			.borders(Borders::ALL)
			.border_set(ratatui::symbols::border::ROUNDED)
			.border_style(Style::default().fg(if core.info_focus == InfoFocus::Details { theme().component.active_pane_border } else { theme().component.pane_border }));

		let info_items: Vec<ListItem> = core.info_details.iter().map(|(k, v)| {
			let key_width = 20;
			let val_width = (chunks[1].width as usize).saturating_sub(key_width + 6);
			let line = Line::from(vec![
				Span::raw(" "), // Extra indentation
				Span::raw(format!("{:<width$}", format!("{}:", k), width = key_width)),
				Span::raw(format!(" {:>width$}", v, width = val_width)),
			]);
			ListItem::new(line).style(Style::default().fg(theme().component.default_text))
		}).collect();

		let info_highlight_style = if core.info_focus == InfoFocus::Details {
			Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
		} else {
			Style::default().add_modifier(Modifier::UNDERLINED)
		};

		let info_list = List::new(info_items)
			.block(info_block)
			.highlight_style(info_highlight_style);

		let mut info_state = ratatui::widgets::ListState::default();
		info_state.select(Some(core.info_details_cursor));
		f.render_stateful_widget(info_list, chunks[1], &mut info_state);
	}
}
