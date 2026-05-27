use pier_config::theme;
use pier_core::core::{ActivePanel, Core};
use pier_core::filetree::FileP4Status;
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Modifier, Style},
  widgets::{Block, Borders, List, ListItem, Paragraph},
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

  let scope_constraint = if state.is_scope_expanded {
    Constraint::Percentage(50)
  } else {
    Constraint::Length(3)
  };

  let left_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      scope_constraint,
      Constraint::Percentage(70),
      Constraint::Min(3),
    ])
    .split(left_area);

  let scope_area = left_chunks[0];
  let filetree_area = left_chunks[1];
  let pending_area = left_chunks[2];

  let right_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Percentage(30),
      Constraint::Percentage(50),
      Constraint::Percentage(20),
    ])
    .split(right_area);

  let changelist_area = right_chunks[0];
  let detail_area = right_chunks[1];
  let log_area = right_chunks[2];

  // Scope
  let scope_block = get_block("[1] Scope ", ActivePanel::Scope, core.active_panel);
  let scope_inner = scope_block.inner(scope_area);
  
  let root_str = core.client_root.to_string_lossy();
  let display_text = if scope_inner.width > 15 {
    let prefix = "Client Root: ";
    let full_text = format!("{}{}", prefix, root_str);
    
    if full_text.len() as u16 <= scope_inner.width {
      full_text
    } else {
      let last_part = core.client_root.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root_str.to_string());
      
      let abbreviated = format!("{}.../{}", prefix, last_part);
      if abbreviated.len() as u16 <= scope_inner.width {
        abbreviated
      } else {
        abbreviated.chars().take(scope_inner.width as usize).collect()
      }
    }
  } else {
    "".to_string()
  };

  f.render_widget(Paragraph::new(display_text).block(scope_block).style(Style::default().fg(theme().component.default_text)), scope_area);
  
  // FileTree
  render_filetree(f, filetree_area, core);

  // Pending
  render_pending(f, pending_area, core);

  // Right Panels
  let cl_block = get_block("[4] ChangeList ", ActivePanel::ChangeList, core.active_panel);
  let mut cl_items: Vec<ListItem> = Vec::new();
  let mut current_ui_index = 0;
  let mut selectable_index = 0;
  let mut selected_ui_index = 0;

  let content_width = (changelist_area.width as usize).saturating_sub(5);
  let is_cl_active = core.active_panel == ActivePanel::ChangeList;

  for (i, cl) in core.changelists.iter().enumerate() {
    let is_expanded = core.expanded_ids.contains(&cl.id);
    let is_head = i == 0;
    let is_selected = core.cl_cursor == selectable_index;
    
    let symbol = if is_selected && is_cl_active {
      if is_head { "󰌕 " } else { "> " }
    } else {
      if is_head { "󰌕 " } else { "  " }
    };

    let id_str = &cl.id;
    let author_str = format!("  {}", cl.author);
    let time_str = &cl.time;
    
    let id_len = id_str.len();
    let author_len = author_str.len();
    let time_len = time_str.len();
    
    let padding = content_width.saturating_sub(id_len).saturating_sub(author_len).saturating_sub(time_len);
    let base_line = format!("{}{}{}{} ", id_str, author_str, " ".repeat(padding), time_str);
    
    cl_items.push(ListItem::new(format!("{}{}", symbol, base_line)));
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
          let file_symbol = if is_cl_active && is_file_selected { "> " } else { "  " };
          
          let file_prefix_str = format!("{}   ", file_symbol);
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
  render_log(f, log_area, core);

  // Footer
  let footer_text = format!(" [Q] Quit | [1-5] Switch Panel | Path: {}", core.filetree.current_path.display());
  let footer = Paragraph::new(footer_text)
    .style(Style::default().fg(theme().component.pane_border));
  f.render_widget(footer, footer_area);
}

fn render_filetree(f: &mut Frame, area: Rect, core: &Core) {
  let ft_block = get_block("[2] FileTree ", ActivePanel::FileTree, core.active_panel);
  let ft_inner_area = ft_block.inner(area);
  f.render_widget(ft_block, area);

  // 增加左右 padding 确保 highlight 不贴边
  let ft_padded_area = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
    .split(ft_inner_area)[1];

  let ft_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
    .split(ft_padded_area);
  
  let parent_items: Vec<ListItem> = core.filetree.parent_files.iter().map(|file| {
    let (icon, color) = if file.is_dir {
      if file.is_empty {
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
        _ => (&theme().icon.own_edit, theme().component.default_text), // Default file icon?
      }
    };
    ListItem::new(format!(" {} {} ", icon, file.name)).style(Style::default().fg(color))
  }).collect();
  
  let current_items: Vec<ListItem> = core.filetree.files.iter().enumerate().map(|(idx, file)| {
    let is_selected = core.filetree.selected == idx;
    let (icon, color) = if file.is_dir {
      if is_selected {
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
        _ => (&theme().icon.own_edit, theme().component.default_text),
      }
    };
    ListItem::new(format!(" {} {} ", icon, file.name)).style(Style::default().fg(color))
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
    .highlight_symbol("> ");

  let mut parent_list_state = ratatui::widgets::ListState::default();
  parent_list_state.select(Some(core.filetree.parent_selected));
  
  let mut current_list_state = ratatui::widgets::ListState::default();
  current_list_state.select(Some(core.filetree.selected));

  f.render_stateful_widget(parent_list, ft_chunks[0], &mut parent_list_state);
  f.render_stateful_widget(current_list, ft_chunks[1], &mut current_list_state);
}

fn render_pending(f: &mut Frame, area: Rect, core: &Core) {
  let block = get_block("[3] Pending ", ActivePanel::Pending, core.active_panel);
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
  let header_symbol = if is_pd_active && core.pending_cursor == 0 { "> " } else { "  " };
  let expand_symbol = if core.is_pending_expanded { "󰅖 " } else { "󰅀 " };
  let header_text = format!(" {}{} Default ", header_symbol, expand_symbol);
  let header_style = if is_pd_active && core.pending_cursor == 0 {
    Style::default().bg(theme().selection.cursor_bg).fg(theme().selection.cursor_fg).add_modifier(Modifier::BOLD)
  } else if !is_pd_active && core.pending_cursor == 0 {
    Style::default().add_modifier(Modifier::UNDERLINED)
  } else {
    Style::default().fg(theme().component.default_text)
  };
  
  items.push(ListItem::new(header_text).style(header_style));

  // Files
  if core.is_pending_expanded {
    for (i, file) in core.pending_files.iter().enumerate() {
      let cursor_idx = i + 1;
      let is_selected = core.pending_cursor == cursor_idx;
      let symbol = if is_pd_active && is_selected { "> " } else { "  " };
      
      let (icon, color) = match file.action.as_str() {
        "add" => (&theme().icon.mark_add, theme().p4.add),
        "edit" => (&theme().icon.own_edit, theme().p4.edit),
        "delete" => (&theme().icon.mark_delete, theme().p4.delete),
        _ => (&theme().icon.own_edit, theme().component.default_text),
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
      
      // 增加缩进，恢复层级感
      let left_content = format!("   {}{} {} ", symbol, icon, filename);
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
  let block = get_block("[@] Log ", ActivePanel::Log, core.active_panel);
  
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
  
  // 这里简单的滚动逻辑：根据 log_cursor 粗略计算 scroll offset
  // 更精准的滚动需要计算换行后的行数，暂时简单处理
  let scroll_offset = core.log_cursor as u16 * 3; 
  
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

    // [CheckoutBy] Header if not empty
    if !detail.checkout_by.trim().is_empty() {
      let checkout_line = format!("  CheckoutBy: {}", detail.checkout_by);
      items.push(ListItem::new(checkout_line).style(Style::default().fg(theme().p4.edit).add_modifier(Modifier::BOLD)));
      
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
