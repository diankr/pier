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

		let title = if core.sync_finished {
			Line::from("─Press any key to close ").alignment(ratatui::layout::Alignment::Center)
		} else {
			Line::from("─Sync Process ").alignment(ratatui::layout::Alignment::Center)
		};

		let progress_block = Block::default()
			.title(title)
			.borders(Borders::ALL)
			.border_set(ratatui::symbols::border::ROUNDED)
			.border_style(Style::default().fg(theme().component.active_pane_border));

		let pct = format!("{:.0}%", core.sync_progress * 100.0);
		let count = format!("{}/{}", core.sync_current, core.sync_total);
		
		let gauge_width = chunks[0].width.saturating_sub(2) as usize;
		let label = if gauge_width > 20 {
			let count_len = count.len();
			let pct_len = pct.len();
			let left_padding = gauge_width / 2 - pct_len / 2;
			let right_padding = gauge_width.saturating_sub(left_padding + pct_len + count_len + 1);
			format!("{}{}{}{}{}", " ".repeat(left_padding), pct, " ".repeat(right_padding), count, " ")
		} else {
			format!("{} {}", pct, count)
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
		let icon_width = 3; 
		let file_name_width = list_width.saturating_sub(icon_width + 2); // +1 space left, +1 space right

		// Rotating bar animation: | / - \
		let arcs = ["   ", "  ", " ", ""];
		let frame_idx = (core.sync_tick / 5) % 4; 

		let items: Vec<ListItem> = core.sync_files.iter().filter(|f| f.status != pier_core::core::SyncFileStatus::Pending).map(|f| {
			let filename = f.depot_path.split('/').last().unwrap_or(&f.depot_path);
			let truncated_name = if filename.chars().count() > file_name_width {
				let mut s: String = filename.chars().take(file_name_width.saturating_sub(3)).collect();
				s.push_str("...");
				s
			} else {
				filename.to_string()
			};
			
			let status_span = match f.status {
				pier_core::core::SyncFileStatus::Pending => Span::raw("   "),
				pier_core::core::SyncFileStatus::Syncing => Span::styled(arcs[frame_idx as usize], Style::default().fg(Color::Yellow)),
				pier_core::core::SyncFileStatus::Done => Span::styled(format!("{:>width$}", theme().icon.check, width = icon_width), Style::default().fg(Color::Green)),
			};
			
			let line = Line::from(vec![
				Span::raw(" "), // Left indentation
				Span::raw(format!("{:<width$}", truncated_name, width = file_name_width)),
				Span::raw(" "),
				status_span,
				Span::raw(" "), // Right indentation
			]);
			ListItem::new(line)
		}).collect();

		let list = List::new(items)
			.block(list_block)
			.style(Style::default().fg(theme().component.default_text))
			.highlight_style(if core.sync_finished { Style::default() } else { Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD) });
		
		let mut state = ratatui::widgets::ListState::default();
		if !core.sync_files.is_empty() && !core.sync_finished {
			state.select(Some(0)); 
		}
		f.render_stateful_widget(list, chunks[1], &mut state);
	}
}
