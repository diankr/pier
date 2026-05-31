use pier_config::theme;
use pier_core::core::ActivePanel;
use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::Style,
	widgets::{Block, Borders},
};

pub(crate) mod domain;
pub(crate) mod filetree;
pub(crate) mod pending;
pub(crate) mod changelist;
pub(crate) mod detail;
pub(crate) mod log;
pub(crate) mod footer;
pub(crate) mod overlays;

pub(crate) use domain::Domain;
pub(crate) use filetree::FileTree;
pub(crate) use pending::Pending;
pub(crate) use changelist::ChangeList;
pub(crate) use detail::Detail;
pub(crate) use log::Log;
pub(crate) use footer::Footer;
pub(crate) use overlays::info::Info;
pub(crate) use overlays::login::Login;
pub(crate) use overlays::submit::Submit;
pub(crate) use overlays::sync::Sync;

pub(crate) fn get_block(title: &'static str, panel: ActivePanel, active_panel: ActivePanel) -> Block<'static> {
  let style = if active_panel == panel {
    Style::default().fg(theme().component.active_pane_border)
  } else {
    Style::default().fg(theme().component.pane_border)
  };
  
  // 左侧用横线填充，右侧保持空出一个字符的空隙
  let padded_title = format!("─{} ", title);
  
  Block::default()
    .title(padded_title)
    .borders(Borders::ALL)
    .border_set(ratatui::symbols::border::ROUNDED)
    .border_style(style)
}

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
  let popup_layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Percentage((100 - percent_y) / 2),
      Constraint::Percentage(percent_y),
      Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

  Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
      Constraint::Percentage((100 - percent_x) / 2),
      Constraint::Percentage(percent_x),
      Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
