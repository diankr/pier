use std::process::Command;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeListItem {
	pub id: String,
	pub author: String,
	pub time: String,      // 格式化后的字符串: 05/11 16:33
	pub description: String,
	pub details: Option<ChangeListDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeListDetail {
	pub full_description: Vec<String>,
	pub files: Vec<ChangeListFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeListFile {
	pub revision: String,
	pub action: String,
	pub path: String,
}

pub fn fetch_changelist_detail(id: &str, root: &Path) -> Result<ChangeListDetail, String> {
	let output = Command::new("p4")
		.arg("describe")
		.arg("-s")
		.arg(id)
		.current_dir(root)
		.output()
		.map_err(|e| format!("Failed to execute p4 describe: {}", e))?;

	if !output.status.success() {
		return Err(format!("Failed to describe changelist {}.", id));
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let mut full_description = Vec::new();
	let mut files = Vec::new();
	let mut section = 0; // 0: header, 1: description, 2: files

	for line in stdout.lines() {
		let trimmed = line.trim();
		if section == 0 && line.is_empty() {
			section = 1;
			continue;
		}
		if section == 1 && (trimmed.starts_with("Affected files ...") || trimmed.starts_with("Differences ...")) {
			section = 2;
			continue;
		}

		match section {
			1 => {
				if !line.is_empty() {
					full_description.push(line.trim_start().to_string());
				}
			}
			2 => {
				if trimmed.starts_with("...") {
					// 格式: ... //depot/path/file.txt#1 add
					let parts: Vec<&str> = trimmed.split_whitespace().collect();
					if parts.len() >= 3 {
						let path_with_rev = parts[1];
						let action = parts[2].to_string();
						
						let sub_parts: Vec<&str> = path_with_rev.split('#').collect();
						if sub_parts.len() >= 2 {
							files.push(ChangeListFile {
								path: sub_parts[0].to_string(),
								revision: format!("#{}", sub_parts[1]),
								action,
							});
						}
					}
				}
			}
			_ => {}
		}
	}

	Ok(ChangeListDetail {
		full_description,
		files,
	})
}

pub fn save_to_cache(items: &[ChangeListItem]) {
	if let Some(cache_dir) = dirs::cache_dir() {
		let pier_cache = cache_dir.join("pier");
		if let Err(_) = fs::create_dir_all(&pier_cache) {
			return;
		}
		let cache_file = pier_cache.join("changelists.json");
		if let Ok(json) = serde_json::to_string_pretty(items) {
			let _ = fs::write(cache_file, json);
		}
	}
}

pub fn fetch_changelists(root: &Path) -> Result<Vec<ChangeListItem>, String> {
	let output = Command::new("p4")
		.arg("changes")
		.arg("-t")
		.arg("-m")
		.arg("10")
		.current_dir(root)
		.output()
		.map_err(|e| format!("Failed to execute p4 changes: {}", e))?;

	if !output.status.success() {
		return Err("Failed to get p4 changes.".to_string());
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let mut items = Vec::new();

	// p4 changes -t 输出格式示例:
	// Change 12345 on 2023/05/11 16:33:14 by user@workspace 'description...'
	for line in stdout.lines() {
		if line.is_empty() {
			continue;
		}

		let parts: Vec<&str> = line.split_whitespace().collect();
		if parts.len() < 7 {
			continue;
		}

		let id = parts[1].to_string();
		let date = parts[3]; // 2023/05/11
		let time_full = parts[4]; // 16:33:14
		let author_full = parts[6]; // user@workspace
		
		let author = author_full.split('@').next().unwrap_or(author_full).to_string();
		
		// 格式化日期：2023/05/11 -> 05/11
		let date_parts: Vec<&str> = date.split('/').collect();
		let formatted_date = if date_parts.len() == 3 {
			format!("{}/{}", date_parts[1], date_parts[2])
		} else {
			date.to_string()
		};

		// 格式化时间：16:33:14 -> 16:33
		let time_parts: Vec<&str> = time_full.split(':').collect();
		let formatted_time = if time_parts.len() >= 2 {
			format!("{}:{}", time_parts[0], time_parts[1])
		} else {
			time_full.to_string()
		};

		let final_time = format!("{} {}", formatted_date, formatted_time);

		// 提取描述
		let description = line.splitn(8, |c: char| c.is_whitespace()).last().unwrap_or("").trim_matches('\'').to_string();

		items.push(ChangeListItem {
			id,
			author,
			time: final_time,
			description,
			details: None,
		});
	}

	save_to_cache(&items);

	Ok(items)
}
