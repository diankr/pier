use pier_config::theme;
use pier_core::core::Core;
use ratatui::{
	layout::{Constraint, Layout, Rect},
	style::{Color, Modifier, Style},
	widgets::{Block, Borders, Gauge, List, ListItem},
	text::{Line, Span},
	Frame,
};
use super::super::centered_rect;

pub(crate) struct Sync;

impl Sync {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let overlay_area = centered_rect(70, 60, area);
		f.render_widget(ratatui::widgets::Clear, overlay_area);

		let chunks = Layout::default()
			.direction(ratatui::layout::Direction::Vertical)
			.constraints([
				Constraint::Length(3), // Sync Process
				Constraint::Min(0),    // File to Sync
			])
			.split(overlay_area);

		// Upper Block: Sync Process
		let progress_block = Block::default()
			.title(Line::from("─Sync Process ").alignment(ratatui::layout::Alignment::Center))
			.borders(Borders::ALL)
			.border_set(ratatui::symbols::border::ROUNDED)
			.border_style(Style::default().fg(theme().component.active_pane_border));

		let label = if core.sync_total_bytes > 0 {
			let pct = format!("{:.1}%", core.sync_progress * 100.0);
			let bytes = format!("{:.1} MB / {:.1} MB", 
				core.sync_synced_bytes as f64 / 1024.0 / 1024.0,
				core.sync_total_bytes as f64 / 1024.0 / 1024.0
			);
			
			let gauge_width = chunks[0].width.saturating_sub(2) as usize;
			let pct_len = pct.len();
			let bytes_len = bytes.len();
			
			if gauge_width > pct_len + bytes_len + 4 {
				let left_padding = (gauge_width - pct_len) / 2;
				let right_padding = gauge_width.saturating_sub(left_padding + pct_len + bytes_len + 1);
				format!("{}{}{}{}", " ".repeat(left_padding), pct, " ".repeat(right_padding), bytes)
			} else {
				format!("{} ({})", pct, bytes)
			}
		} else {
			format!("{:.0}%", core.sync_progress * 100.0)
		};
		let gauge = Gauge::default()
			.block(progress_block)
			.gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black).add_modifier(Modifier::ITALIC))
			.ratio(core.sync_progress)
			.label(label);
		f.render_widget(gauge, chunks[0]);

		// Lower Block: File to Sync
		let list_block = Block::default()
			.title(Line::from("─File to Sync ").alignment(ratatui::layout::Alignment::Center))
			.borders(Borders::ALL)
			.border_set(ratatui::symbols::border::ROUNDED)
			.border_style(Style::default().fg(theme().component.pane_border));

		let list_width = chunks[1].width.saturating_sub(4) as usize;
		let progress_width = 20;
		let file_name_width = list_width.saturating_sub(progress_width + 1);

		let items: Vec<ListItem> = core.sync_files.iter().map(|f| {
			let filename = f.depot_path.split('/').last().unwrap_or(&f.depot_path);
			let truncated_name = if filename.chars().count() > file_name_width {
				let mut s: String = filename.chars().take(file_name_width.saturating_sub(3)).collect();
				s.push_str("...");
				s
			} else {
				filename.to_string()
			};
			
			let ratio = if f.size > 0 {
				(f.synced as f64 / f.size as f64).min(1.0)
			} else {
				0.0
			};
			
			let filled = (ratio * progress_width as f64).round() as usize;
			let empty = progress_width.saturating_sub(filled);
			let bar = format!("[{}{}]", "▪".repeat(filled), " ".repeat(empty));
			
			let line = Line::from(vec![
				Span::raw(format!("{:<width$}", truncated_name, width = file_name_width)),
				Span::raw(" "),
				Span::styled(bar, Style::default().fg(Color::Yellow)),
			]);
			ListItem::new(line)
		}).collect();

		let list = List::new(items)
			.block(list_block)
			.style(Style::default().fg(theme().component.default_text))
			.highlight_style(Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD));
		
		let mut state = ratatui::widgets::ListState::default();
		if core.sync_current > 0 {
			state.select(Some(core.sync_current.saturating_sub(1)));
		}
		f.render_stateful_widget(list, chunks[1], &mut state);
	}
}
