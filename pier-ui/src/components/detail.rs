use pier_config::theme;
use pier_core::core::{ActivePanel, Core};
use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Modifier, Style},
	widgets::{List, ListItem, Paragraph},
	text::{Line, Span},
	Frame,
};
use super::get_block;

pub(crate) struct Detail;

impl Detail {
	pub(crate) fn render(f: &mut Frame, area: Rect, core: &Core) {
		let block = get_block("[Tab] Detail ", ActivePanel::Detail, core.active_panel);
		let inner = block.inner(area);
		f.render_widget(block, area);

		let is_dt_active = core.active_panel == ActivePanel::Detail;

		if let Some(detail) = &core.current_detail {
			let mut items = Vec::new();
			let content_width = (inner.width as usize).saturating_sub(4);

			let checkout_by = detail.checkout_by.trim();

			// [CheckoutBy] Header if not empty
			if !checkout_by.is_empty() {
				let checkout_label = "CheckoutBy:";
				let checkout_val = checkout_by;
				
				let pad_len = content_width.saturating_sub(checkout_label.len()).saturating_sub(checkout_val.len());
				let line = Line::from(vec![
					Span::raw("  "),
					Span::styled(checkout_label, Style::default().fg(theme().p4.edit).add_modifier(Modifier::BOLD)),
					Span::raw(" ".repeat(pad_len)),
					Span::styled(checkout_val, Style::default().fg(theme().p4.edit).add_modifier(Modifier::BOLD)),
				]);
				
				items.push(ListItem::new(line));
				
				let separator = "─".repeat(content_width);
				items.push(ListItem::new(format!("  {}", separator)).style(Style::default().fg(theme().component.pane_border)));
			}

			let labels = [
				"FileName", "FileSize", "DepotPath", "Revision", 
				"DateModified", "ChangeList", "Action", "LatestUser"
			];
			let values = [
				&detail.filename, &detail.filesize, &detail.depot_path, &detail.revision,
				&detail.date_modified, &detail.changelist, &detail.action, &detail.latest_user
			];

			for (i, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
				let is_selected = is_dt_active && core.detail_cursor == i;
				let symbol = if is_selected { "> " } else { "  " };
				
				let padding = content_width.saturating_sub(label.len()).saturating_sub(value.len());
				let line = format!("{}{} {}{}", symbol, label, " ".repeat(padding), value);
				let mut list_item = ListItem::new(line);
				
				if is_selected {
					list_item = list_item.style(Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD));
				} else {
					list_item = list_item.style(Style::default().fg(theme().component.default_text));
				}
				items.push(list_item);
			}

			let list = List::new(items);
			f.render_widget(list, inner);
		} else if let Some(err) = &core.detail_error {
			let text = if err.contains("Not a Perforce-managed object") {
				"Not a Perforce-managed object"
			} else {
				err
			};
			let p = Paragraph::new(text)
				.style(Style::default().fg(theme().component.pane_border))
				.alignment(ratatui::layout::Alignment::Center);
			
			let vertical_chunks = Layout::default()
				.direction(Direction::Vertical)
				.constraints([
					Constraint::Percentage(45),
					Constraint::Min(1),
					Constraint::Percentage(45),
				])
				.split(inner);
			
			f.render_widget(p, vertical_chunks[1]);
		}
	}
}
