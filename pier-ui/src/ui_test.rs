use crossterm::{
	event::{self, KeyCode, KeyEventKind},
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
	ExecutableCommand,
};
use ratatui::{prelude::*, widgets::Paragraph, widgets::Block, widgets::Borders};
use std::io::{self, stdout};

pub fn draw_ui() -> io::Result<()> {
	// 1. 初始化终端 (知识点：Result 错误处理)
	// enable_raw_mode 会让终端不再自动回显字符，方便我们监听按键
	stdout().execute(EnterAlternateScreen)?; 
	enable_raw_mode()?;
	
	// 使用 crossterm 作为后端创建 Ratatui 终端实例
	let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

	// 2. 简单的渲染循环
	loop {
		// 渲染界面 (知识点：闭包 Closure)
		terminal.draw(|frame| {
			let area = frame.size();
			// 渲染一个简单的段落组件
			frame.render_widget(
				Paragraph::new("P4 TUI - 按 'q' 退出")
					.blue()
					.block(Block::default().title("状态").borders(Borders::ALL)),
				area,
			);
		})?;

		// 3. 事件处理 (知识点：匹配模式 Match & 枚举 Enums)
		if event::poll(std::time::Duration::from_millis(16))? {
			if let event::Event::Key(key) = event::read()? {
				// 只有在按下瞬间触发 (防止 Windows 下长按重复触发)
				if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
					break;
				}
			}
		}
	}

	// 4. 恢复终端环境 (非常重要！)
	disable_raw_mode()?;
	stdout().execute(LeaveAlternateScreen)?;
	Ok(())
}
