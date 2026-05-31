use pier_config::theme;
use pier_core::core::{ActivePanel, Core, SubmitFocus};
use pier_core::filetree::FileP4Status;
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect, Alignment},
  style::{Modifier, Style, Color},
  widgets::{Block, Borders, List, ListItem, Paragraph, Gauge},
  text::{Line, Span},
  Frame,
};

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

  let is_scope_active = core.active_panel == ActivePanel::Scope;
  let is_ft_active = core.active_panel == ActivePanel::FileTree;
  let is_pd_active = core.active_panel == ActivePanel::Pending;
  let is_cl_active = core.active_panel == ActivePanel::ChangeList;
  let is_dt_active = core.active_panel == ActivePanel::Detail;
  let is_log_active = core.active_panel == ActivePanel::Log;

  let scope_constraint = if state.is_scope_expanded {
    Constraint::Percentage(50)
  } else {
    Constraint::Length(3)
  };

  let pending_constraint = if is_pd_active {
    Constraint::Length(20) // Active 时两倍高度 (假设基础 6)
  } else {
    Constraint::Length(10)
  };

  let left_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      scope_constraint,     // Domain
      Constraint::Min(10),  // FileTree (弹性)
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
    let log_height = 10; // 维持跟 Pending inactive 一样高
    let right_parts = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Min(10), // CL + Detail
        Constraint::Length(log_height), // Log
      ])
      .split(right_area);
    
    let cl_constraint = if is_cl_active {
      Constraint::Percentage(66) // Active 时两倍于 Detail
    } else {
      Constraint::Percentage(50) // 默认二者平分
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

  // Domain (Domain 改名及内容 margin)
  let scope_block = get_block("[1] Domain", ActivePanel::Scope, core.active_panel);
  let scope_inner = scope_block.inner(scope_area);
  
  // 增加左侧 1 字符 margin
  let scope_padded_area = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Length(1), Constraint::Min(0)])
    .split(scope_inner)[1];

  let root_str = core.virtual_root.as_ref()
    .map(|vr| vr.to_string_lossy())
    .unwrap_or_else(|| core.client_root.to_string_lossy());

  let is_virtual = core.virtual_root.is_some();
  let prefix = if is_virtual { "Virtual Root: " } else { "Client Root: " };

  let display_text = if scope_padded_area.width > 15 {
    let full_text = format!("{}{}", prefix, root_str);
    
    if full_text.len() as u16 <= scope_padded_area.width {
      full_text
    } else {
      let last_part = if is_virtual {
        core.virtual_root.as_ref().and_then(|vr| vr.file_name())
      } else {
        core.client_root.file_name()
      }
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_else(|| root_str.to_string());
      
      let abbreviated = format!("{}.../{}", prefix, last_part);
      if abbreviated.len() as u16 <= scope_padded_area.width {
        abbreviated
      } else {
        abbreviated.chars().take(scope_padded_area.width as usize).collect()
      }
    }
  } else {
    "".to_string()
  };

  f.render_widget(scope_block, scope_area);
  f.render_widget(Paragraph::new(display_text).style(Style::default().fg(theme().component.default_text)), scope_padded_area);
  
  // FileTree
  render_filetree(f, filetree_area, core);

  // Pending
  render_pending(f, pending_area, core);

  // Right Panels (Changelist & Detail 只在不被 Log 全屏时显示)
  if !is_log_active {
    let cl_block = get_block("[4] ChangeList", ActivePanel::ChangeList, core.active_panel);
    let mut cl_items: Vec<ListItem> = Vec::new();
    let mut current_ui_index = 0;
    let mut selectable_index = 0;
    let mut selected_ui_index = 0;

    let content_width = (changelist_area.width as usize).saturating_sub(5);

  for (i, cl) in core.changelists.iter().enumerate() {
    let is_expanded = core.expanded_ids.contains(&cl.id);
    let is_head = i == 0;
    let is_selected = core.cl_cursor == selectable_index;
    
    let is_new = if let Some(synced_id) = &core.synced_change_id {
      let cl_val = cl.id.parse::<i64>().unwrap_or(0);
      let synced_val = synced_id.parse::<i64>().unwrap_or(0);
      cl_val > synced_val
    } else {
      false
    };

    let base_style = if is_new {
      Style::default().fg(Color::Red)
    } else {
      Style::default().fg(theme().component.default_text)
    };

    let is_synced = if let Some(synced_id) = &core.synced_change_id {
      cl.id == *synced_id
    } else {
      false
    };

    let symbol = if is_synced {
      format!(" {}", theme().icon.changelist_head)
    } else if is_head {
      format!(" \u{f0e95}") // \uf0e9 for server head
    } else {
      "  ".to_string()
    };

    
    let icon_span = if is_head {
      ratatui::text::Span::styled(symbol, base_style)
    } else {
      ratatui::text::Span::styled(symbol, base_style)
    };

    let id_str = format!(" {} ", cl.id);
    let author_str = format!("  {}", cl.author);
    let time_str = &cl.time;
    
    let id_len = id_str.len();
    let author_len = author_str.len();
    let time_len = time_str.len();
    
    let padding = content_width.saturating_sub(id_len).saturating_sub(author_len).saturating_sub(time_len);
    
    cl_items.push(ListItem::new(ratatui::text::Line::from(vec![
      icon_span, 
      ratatui::text::Span::styled(id_str, base_style.add_modifier(Modifier::BOLD)),
      ratatui::text::Span::styled(author_str, base_style),
      ratatui::text::Span::styled(" ".repeat(padding), base_style),
      ratatui::text::Span::styled(time_str, base_style),
    ])));
    if is_selected {
      selected_ui_index = current_ui_index;
    }
    current_ui_index += 1;
    selectable_index += 1;

    if is_expanded {
      if let Some(details) = &cl.details {
        let detail_prefix = "     "; 
        let detail_content_width = content_width.saturating_sub(3);

        for desc_line in &details.full_description {
          cl_items.push(ListItem::new(format!("{}{}", detail_prefix, desc_line)).style(Style::default().fg(theme().component.pane_border)));
          current_ui_index += 1;
        }

        let separator = "─".repeat(detail_content_width);
        cl_items.push(ListItem::new(format!("{}{}", detail_prefix, separator)).style(Style::default().fg(theme().component.pane_border)));
        current_ui_index += 1;

        for (_f_idx, file) in details.files.iter().enumerate() {
          let is_file_selected = core.cl_cursor == selectable_index;
          
          let file_prefix_str = "      ";
          let file_info = format!("{} | {} | ", file.revision, file.action);
          
          let display_path = file.path.replacen("//depot", "...", 1);
          let file_info_len = file_info.chars().count();
          
          let avail_path_width = detail_content_width.saturating_sub(file_info_len);
          let path_len = display_path.chars().count();
          
          let file_line = if path_len <= avail_path_width {
            let path_padding = avail_path_width.saturating_sub(path_len);
            format!("{}{}{}{}{} ", file_prefix_str, file_info, " ".repeat(path_padding), display_path, " ")
          } else {
            format!("{}{}{} ", file_prefix_str, file_info, display_path)
          };

          cl_items.push(ListItem::new(file_line).style(Style::default().fg(theme().component.default_text)));
          if is_file_selected {
            selected_ui_index = current_ui_index;
          }
          current_ui_index += 1;
          selectable_index += 1;
        }
      }
    }
  }

  let highlight_style = if is_cl_active {
    Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
  } else {
    Style::default().add_modifier(Modifier::UNDERLINED)
  };

  let cl_list = List::new(cl_items).highlight_style(highlight_style).style(Style::default().fg(theme().component.default_text));

  let mut cl_list_state = ratatui::widgets::ListState::default();
  cl_list_state.select(Some(selected_ui_index));

  f.render_stateful_widget(cl_list.block(cl_block), changelist_area, &mut cl_list_state);

  render_detail(f, detail_area, core);
  } // 这里闭合 if !is_log_active

  render_log(f, log_area, core);

  // Footer
  let left_hints = match core.active_panel {
    ActivePanel::Scope      => "[Enter] p4 info",
    ActivePanel::FileTree   => "[c] checkout | [a] add | [d] delete | [r] revert",
    ActivePanel::Pending    => "[S] submit | [r] revert",
    ActivePanel::ChangeList => "[s] fetch & sync | [f] fetch | [g] sync to selected | [F] show in filetree",

    ActivePanel::Detail     => "[y] copy to clipboard",
    _ => "",
  };
  
  let right_fixed = "[Q] quit | [?] keybind | ver 0.0.1";
  let total_width = footer_area.width as usize;
  let right_width = right_fixed.chars().count();
  
  let mut footer_line = String::new();
  if total_width > right_width + 5 {
    let avail_left = total_width.saturating_sub(right_width).saturating_sub(2);
    let left_part = if left_hints.chars().count() > avail_left {
      format!("{}...", &left_hints[..avail_left.saturating_sub(3)])
    } else {
      left_hints.to_string()
    };
    let spacing = total_width.saturating_sub(left_part.chars().count()).saturating_sub(right_width);
    footer_line = format!("{}{}{}", left_part, " ".repeat(spacing), right_fixed);
  } else {
    footer_line = right_fixed.to_string();
  }

  let footer = Paragraph::new(footer_line)
    .style(Style::default().fg(theme().component.pane_border));
  f.render_widget(footer, footer_area);
  
  if core.is_submit_overlay_open {
    render_submit_overlay(f, area, core);
  }
  
  if core.is_login_overlay_open {
    render_login_overlay(f, area, core);
  }

  if core.is_syncing {
    render_sync_overlay(f, area, core);
  }
}

fn render_login_overlay(f: &mut Frame, area: Rect, core: &Core) {
  let mut overlay_area = centered_rect(60, 25, area);
  if overlay_area.height < 12 {
    overlay_area.height = 12.min(area.height);
    overlay_area.y = (area.height.saturating_sub(overlay_area.height)) / 2;
  }
  f.render_widget(ratatui::widgets::Clear, overlay_area);

  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(3), // Login (password)
      Constraint::Min(6),    // Info
    ])
    .split(overlay_area);

  // Upper Block: p4 login
  let login_block = Block::default()
    .title(Line::from("─p4 login ").alignment(Alignment::Center))
    .borders(Borders::ALL)
    .border_set(ratatui::symbols::border::ROUNDED)
    .border_style(Style::default().fg(theme().component.active_pane_border));

  let masked_password = "*".repeat(core.login_password.len());
  let p = Paragraph::new(masked_password).block(login_block).style(Style::default().fg(theme().component.default_text));
  f.render_widget(p, chunks[0]);

  // Cursor for password
  let cursor_x = chunks[0].x + 1 + core.login_password.len() as u16;
  let cursor_y = chunks[0].y + 1;
  let max_x = chunks[0].x + chunks[0].width - 2;
  f.set_cursor_position((cursor_x.min(max_x), cursor_y));

  // Lower Block: info
  let info_block = Block::default()
    .title(Line::from("─info ").alignment(Alignment::Center))
    .borders(Borders::ALL)
    .border_set(ratatui::symbols::border::ROUNDED)
    .border_style(Style::default().fg(theme().component.pane_border));
  
  let info_style = if core.login_info == "Password Invalid" {
    Style::default().fg(Color::Red)
  } else {
    Style::default().fg(theme().component.default_text)
  };

  let content_width = chunks[1].width.saturating_sub(2);
  let mut info_content = vec![
    Line::from(vec![Span::raw(" "), Span::styled(&core.login_info, info_style)]),
    Line::from("─".repeat(content_width as usize)).style(Style::default().fg(theme().component.pane_border)),
  ];

  // user:value (left and right aligned with 1-char margin)
  let user_label = "user";
  let user_value = &core.login_user;
  let user_padding = content_width.saturating_sub(user_label.len() as u16).saturating_sub(user_value.len() as u16).saturating_sub(2);
  info_content.push(Line::from(vec![
    Span::raw(" "),
    Span::raw(user_label),
    Span::raw(" ".repeat(user_padding as usize)),
    Span::raw(user_value),
    Span::raw(" "),
  ]));

  // target server:value
  let server_label = "target server";
  let server_value = &core.login_server;
  let server_padding = content_width.saturating_sub(server_label.len() as u16).saturating_sub(server_value.len() as u16).saturating_sub(2);
  info_content.push(Line::from(vec![
    Span::raw(" "),
    Span::raw(server_label),
    Span::raw(" ".repeat(server_padding as usize)),
    Span::raw(server_value),
    Span::raw(" "),
  ]));

  let p_info = Paragraph::new(info_content).block(info_block).style(Style::default().fg(theme().component.default_text));
  f.render_widget(p_info, chunks[1]);
}

fn render_filetree(f: &mut Frame, area: Rect, core: &Core) {
  let ft_block = get_block("[2] FileTree", ActivePanel::FileTree, core.active_panel);
  let ft_inner_area = ft_block.inner(area);
  f.render_widget(ft_block, area);

  // 增加左右 padding 确保 highlight 不贴边
  let ft_padded_area = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
    .split(ft_inner_area)[1];

  let ft_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
      Constraint::Percentage(30),
      Constraint::Length(1), // 增加一个字符的间隔
      Constraint::Percentage(70)
    ])
    .split(ft_padded_area);
  
  let parent_items: Vec<ListItem> = core.filetree.parent_files.iter().map(|file| {
    let (icon, color) = if file.is_dir {
      if file.path == core.client_root {
        (&theme().icon.client_root, theme().component.default_text)
      } else {
        (&theme().icon.folder_open, theme().component.default_text)
      }
    } else {
      match file.p4_status {
        FileP4Status::Add => (&theme().icon.mark_add, theme().p4.add),
        FileP4Status::Edit => (&theme().icon.own_edit, theme().p4.edit),
        FileP4Status::Delete => (&theme().icon.mark_delete, theme().p4.delete),
        FileP4Status::OtherCheckout => (&theme().icon.other_checkout, theme().p4.other_checkout),
        FileP4Status::Untracked => (&theme().icon.untracked, theme().component.default_text),
        _ => (&theme().icon.file_default, theme().component.default_text),
      }
    };
    ListItem::new(format!(" {} {} ", icon, file.name)).style(Style::default().fg(color))
  }).collect();
  
  let current_items: Vec<ListItem> = core.filetree.files.iter().enumerate().map(|(idx, file)| {
    let is_selected = core.filetree.selected == idx;
    let (icon, color) = if file.is_dir {
      if file.path == core.client_root {
        (&theme().icon.client_root, theme().component.default_text)
      } else if is_selected {
        (&theme().icon.folder_open, theme().component.default_text)
      } else if file.is_empty {
        (&theme().icon.folder_empty, theme().component.default_text)
      } else {
        (&theme().icon.folder, theme().component.default_text)
      }
    } else {
      match file.p4_status {
        FileP4Status::Add => (&theme().icon.mark_add, theme().p4.add),
        FileP4Status::Edit => (&theme().icon.own_edit, theme().p4.edit),
        FileP4Status::Delete => (&theme().icon.mark_delete, theme().p4.delete),
        FileP4Status::OtherCheckout => (&theme().icon.other_checkout, theme().p4.other_checkout),
        FileP4Status::Untracked => (&theme().icon.untracked, theme().component.default_text),
        _ => (&theme().icon.file_default, theme().component.default_text),
      }
    };
    
    // 如果未被高亮选中，在原本 ">" 的位置显示对应颜色的 1/2 宽实心块
    let status_block = if is_selected {
      "  " 
    } else {
      match file.p4_status {
        FileP4Status::Add | FileP4Status::Edit | FileP4Status::Delete | FileP4Status::OtherCheckout => "▌ ",
        _ => "  ",
      }
    };
    
    let block_style = if is_selected { Style::default() } else { Style::default().fg(color) };
    
    let line = ratatui::text::Line::from(vec![
      ratatui::text::Span::styled(status_block, block_style),
      ratatui::text::Span::styled(format!("{} {} ", icon, file.name), Style::default().fg(color))
    ]);
    
    ListItem::new(line)
  }).collect();

  let is_ft_active = core.active_panel == ActivePanel::FileTree;

  let parent_highlight_style = if is_ft_active {
    Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
  } else {
    Style::default().add_modifier(Modifier::UNDERLINED)
  };

  let parent_list = List::new(parent_items)
    .style(Style::default().fg(theme().component.default_text))
    .highlight_style(parent_highlight_style)
    .highlight_symbol("");

  let current_highlight_style = if is_ft_active {
    Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
  } else {
    Style::default().add_modifier(Modifier::UNDERLINED)
  };

  let current_list = List::new(current_items)
    .style(Style::default().fg(theme().component.default_text))
    .highlight_style(current_highlight_style)
    .highlight_symbol("");

  let mut parent_list_state = ratatui::widgets::ListState::default();
  parent_list_state.select(Some(core.filetree.parent_selected));
  
  let mut current_list_state = ratatui::widgets::ListState::default();
  current_list_state.select(Some(core.filetree.selected));

  f.render_stateful_widget(parent_list, ft_chunks[0], &mut parent_list_state);
  f.render_stateful_widget(current_list, ft_chunks[2], &mut current_list_state);
}

fn render_pending(f: &mut Frame, area: Rect, core: &Core) {
  let block = get_block("[3] Pending", ActivePanel::Pending, core.active_panel);
  let inner = block.inner(area);
  f.render_widget(block, area);

  // 增加左右 padding
  let padded_inner = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
    .split(inner)[1];

  let mut items = Vec::new();
  let is_pd_active = core.active_panel == ActivePanel::Pending;
  
  // Default Changelist Header
  let toggle_symbol = if core.is_pending_expanded { "v" } else { ">" };
  
  let header_icon_span = ratatui::text::Span::styled(&theme().icon.pending_default, Style::default());
  let header_line = ratatui::text::Line::from(vec![
    ratatui::text::Span::from(format!("{} ", toggle_symbol)),
    header_icon_span,
    ratatui::text::Span::from(" Default ")
  ]);

  let header_style = if is_pd_active && core.pending_cursor == 0 {
    Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
  } else if !is_pd_active && core.pending_cursor == 0 {
    Style::default().add_modifier(Modifier::UNDERLINED)
  } else {
    Style::default().fg(theme().component.default_text)
  };
  
  items.push(ListItem::new(header_line).style(header_style));

  // Files
  if core.is_pending_expanded {
    for (i, file) in core.pending_files.iter().enumerate() {
      let cursor_idx = i + 1;
      let is_selected = core.pending_cursor == cursor_idx;
      let symbol = if is_pd_active && is_selected { " " } else { " " };
      
      let (icon, color) = match file.action.as_str() {
        "add" => (&theme().icon.mark_add, theme().p4.add),
        "edit" => (&theme().icon.own_edit, theme().p4.edit),
        "delete" => (&theme().icon.mark_delete, theme().p4.delete),
        _ => (&theme().icon.file_default, theme().component.default_text),
      };
      
      let filename = std::path::Path::new(&file.path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file.path.clone());
      
      let parent_path = std::path::Path::new(&file.path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
      let display_path = parent_path.replacen("//depot", "...", 1);
      
      // 增加缩进，恢复层级感 (Indentation adjusted)
      let left_content = format!("  {}{} {} ", symbol, icon, filename);
      let right_content = format!("{}  {}/ ", file.revision, display_path);
      
      let left_len = left_content.chars().count();
      let right_len = right_content.chars().count();
      let total_width = padded_inner.width as usize;
      
      let mut line_spans = vec![
        ratatui::text::Span::styled(left_content, Style::default().fg(color))
      ];
      
      if total_width > left_len {
        let avail_right = total_width.saturating_sub(left_len);
        let final_right = if right_len > avail_right {
          format!("{}...", &right_content[..avail_right.saturating_sub(3)])
        } else {
          let padding = avail_right.saturating_sub(right_len);
          format!("{}{}", " ".repeat(padding), right_content)
        };
        line_spans.push(ratatui::text::Span::styled(final_right, Style::default().fg(theme().component.pane_border)));
      }
      
      items.push(ListItem::new(ratatui::text::Line::from(line_spans)));
    }
  }

  let list = List::new(items)
    .style(Style::default().fg(theme().component.default_text));
  
  // 处理选中状态的逻辑移到渲染这里
  let mut state = ratatui::widgets::ListState::default();
  state.select(Some(core.pending_cursor));

  let highlight_style = if is_pd_active {
    Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
  } else if core.pending_cursor != usize::MAX {
    Style::default().add_modifier(Modifier::UNDERLINED)
  } else {
    Style::default()
  };

  f.render_stateful_widget(list.highlight_style(highlight_style), padded_inner, &mut state);
}

fn render_log(f: &mut Frame, area: Rect, core: &Core) {
  let block = get_block("[@] Log", ActivePanel::Log, core.active_panel);
  
  let is_log_active = core.active_panel == ActivePanel::Log;

  // 重新实现 Log 渲染，使用 Paragraph 以支持 Wrap 和滚动
  let mut log_content = Vec::new();
  for (i, log) in core.logs.iter().enumerate() {
    let is_selected = is_log_active && core.log_cursor == i;
    let header_style = if is_selected {
      Style::default().fg(theme().component.active_pane_border).add_modifier(Modifier::BOLD)
    } else {
      Style::default().fg(theme().component.pane_border)
    };

    log_content.push(ratatui::text::Line::from(vec![
      ratatui::text::Span::styled(format!("[{}]", log.time), header_style)
    ]));
    log_content.push(ratatui::text::Line::from(vec![
      ratatui::text::Span::styled(format!("> {}", log.command), Style::default().fg(theme().component.default_text))
    ]));
    
    for line in log.output.lines() {
      log_content.push(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(format!("  {}", line), Style::default().fg(theme().component.pane_border))
      ]));
    }
    log_content.push(ratatui::text::Line::from("")); // Spacer
  }

  let paragraph = Paragraph::new(log_content)
    .style(Style::default().fg(theme().component.default_text))
    .block(block)
    .wrap(ratatui::widgets::Wrap { trim: true });
  
  // 更加精准的滚动：根据 log_cursor 前面的日志所占的行数计算 offset
  let mut scroll_offset = 0;
  for (i, log) in core.logs.iter().enumerate() {
    if i >= core.log_cursor { break; }
    let lines_count = log.output.lines().count() as u16;
    scroll_offset += 1 + 1 + lines_count + 1; // Time + Cmd + Output + Spacer
  }
  
  f.render_widget(paragraph.scroll((scroll_offset, 0)), area);
}

fn render_detail(f: &mut Frame, area: Rect, core: &Core) {
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

fn get_block(title: &'static str, panel: ActivePanel, active_panel: ActivePanel) -> Block<'static> {
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

fn render_submit_overlay(f: &mut Frame, area: Rect, core: &Core) {
  let overlay_area = centered_rect(70, 45, area);
  f.render_widget(ratatui::widgets::Clear, overlay_area);
  
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(3), // Description
      Constraint::Length(7), // File List (5 items + 2 borders)
    ])
    .split(overlay_area);
    
  // Description Block
  let desc_style = if core.submit_focus == SubmitFocus::Description {
    Style::default().fg(theme().component.active_pane_border)
  } else {
    Style::default().fg(theme().component.pane_border)
  };
  let desc_title = format!("─Description ({}) ", core.submit_description.len());
  let desc_block = Block::default()
    .title(desc_title)
    .borders(Borders::ALL)
    .border_set(ratatui::symbols::border::ROUNDED)
    .border_style(desc_style);
  
  let p = Paragraph::new(core.submit_description.as_str()).block(desc_block).style(Style::default().fg(theme().component.default_text));
  f.render_widget(p, chunks[0]);
  
  // Add blinking cursor
  if core.submit_focus == SubmitFocus::Description {
    let cursor_x = chunks[0].x + 1 + core.submit_description.len() as u16;
    let cursor_y = chunks[0].y + 1;
    let max_x = chunks[0].x + chunks[0].width - 2;
    f.set_cursor_position((cursor_x.min(max_x), cursor_y));
  }
  
  // File List Block
  let list_style = if core.submit_focus == SubmitFocus::FileList {
    Style::default().fg(theme().component.active_pane_border)
  } else {
    Style::default().fg(theme().component.pane_border)
  };
  let list_block = Block::default()
    .title("─Files to Submit (tab to toggle view) ")
    .borders(Borders::ALL)
    .border_set(ratatui::symbols::border::ROUNDED)
    .border_style(list_style);
    
  let list_inner = chunks[1].inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
  f.render_widget(list_block, chunks[1]);
  
  let mut items = Vec::new();
  for (i, file) in core.pending_files.iter().enumerate() {
    let is_selected = core.submit_cursor == i;
    let symbol = " "; 
    
    let (icon, color) = match file.action.as_str() {
      "add" => (&theme().icon.mark_add, theme().p4.add),
      "edit" => (&theme().icon.own_edit, theme().p4.edit),
      "delete" => (&theme().icon.mark_delete, theme().p4.delete),
      _ => (&theme().icon.file_default, theme().component.default_text),
    };
    
    let filename = std::path::Path::new(&file.path)
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_else(|| file.path.clone());
    
    let parent_path = std::path::Path::new(&file.path)
      .parent()
      .map(|p| p.to_string_lossy().to_string())
      .unwrap_or_default();
    let display_path = parent_path.replacen("//depot", "...", 1);
    
    // 缩进朝左移一个字符
    let left_content = format!("  {}{} {} ", symbol, icon, filename);
    let right_content = format!("{}  {}/ ", file.revision, display_path);
    
    let total_width = list_inner.width as usize;
    let left_len = left_content.chars().count();
    let right_len = right_content.chars().count();

    let mut line_spans = vec![ratatui::text::Span::styled(left_content, Style::default().fg(color))];
    
    if total_width > left_len {
      let avail_right = total_width.saturating_sub(left_len);
      let final_right = if right_len > avail_right {
        format!("{}...", &right_content[..avail_right.saturating_sub(3)])
      } else {
        let padding = avail_right.saturating_sub(right_len);
        format!("{}{}", " ".repeat(padding), right_content)
      };
      line_spans.push(ratatui::text::Span::styled(final_right, Style::default().fg(theme().component.pane_border)));
    }
    
    items.push(ListItem::new(ratatui::text::Line::from(line_spans)));
  }
  
  let list = List::new(items);
  let mut list_state = ratatui::widgets::ListState::default();
  list_state.select(Some(core.submit_cursor));
  
  let highlight_style = if core.submit_focus == SubmitFocus::FileList {
    Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
  } else {
    Style::default()
  };
  
  f.render_stateful_widget(list.highlight_style(highlight_style), list_inner, &mut list_state);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn render_sync_overlay(f: &mut Frame, area: Rect, core: &Core) {
  let overlay_area = centered_rect(70, 60, area);
  f.render_widget(ratatui::widgets::Clear, overlay_area);

  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(3), // Sync Process
      Constraint::Min(0),    // File to Sync
    ])
    .split(overlay_area);

  // Upper Block: Sync Process
  let progress_block = Block::default()
    .title(Line::from("─Sync Process ").alignment(Alignment::Center))
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
    .title(Line::from("─File to Sync ").alignment(Alignment::Center))
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
