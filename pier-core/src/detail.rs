use std::process::Command;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::fs;
use chrono::{TimeZone, Local};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDetail {
	pub filename: String,
	pub filesize: String,
	pub depot_path: String,
	pub revision: String,
	pub date_modified: String,
	pub changelist: String,
	pub action: String,
	pub latest_user: String,
	pub checkout_by: String,
}

pub fn fetch_file_detail(path: &Path) -> Result<FileDetail, String> {
	// 尝试从磁盘缓存获取
	if let Some(cached) = load_from_cache(path) {
		return Ok(cached);
	}

	// 使用 p4 fstat 获取大部分信息
	let output = Command::new("p4")
		.arg("fstat")
		.arg("-Of") // 确保包含 fileSize
		.arg(path)
		.output()
		.map_err(|e| format!("Failed to execute p4 fstat: {}", e))?;

	if !output.status.success() {
		return Err("Not a Perforce-managed object".to_string());
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let mut detail = FileDetail {
		filename: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
		filesize: "0 B".to_string(),
		depot_path: "".to_string(),
		revision: "".to_string(),
		date_modified: "".to_string(),
		changelist: "".to_string(),
		action: "".to_string(),
		latest_user: "".to_string(),
		checkout_by: "".to_string(),
	};

	let mut have_rev = "0".to_string();
	let mut head_rev = "0".to_string();

	for line in stdout.lines() {
		let trimmed = line.trim();
		if !trimmed.starts_with("...") { continue; }
		
		let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
		if parts.len() < 3 { continue; }
		let key = parts[1];
		let value = parts[2];

		match key {
			"depotFile" => {
				// 格式化 DepotPath: 去掉文件名，替换前缀
				let p = Path::new(value);
				let parent = p.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
				detail.depot_path = parent.replacen("//depot/", "...", 1);
				if !detail.depot_path.ends_with('/') {
					detail.depot_path.push('/');
				}
			}
			"headRev" => head_rev = value.to_string(),
			"haveRev" => have_rev = value.to_string(),
			"fileSize" => {
				let size_bytes: u64 = value.parse().unwrap_or(0);
				detail.filesize = format_size(size_bytes);
			}
			"headTime" => {
				let timestamp: i64 = value.parse().unwrap_or(0);
				detail.date_modified = format_time(timestamp);
			}
			"headChange" => detail.changelist = value.to_string(),
			"headAction" => detail.action = value.to_string(),
			"otherOpen0" => detail.checkout_by = value.to_string(),
			"action" => {
				if detail.checkout_by.is_empty() || detail.checkout_by == "Unknown" {
					detail.checkout_by = "You".to_string();
				}
			}
			_ => {}
		}
	}
	
	// Revision 格式化: #have/head
	detail.revision = format!("#{}", head_rev);
	if have_rev != "0" {
		detail.revision = format!("#{} / #{}", have_rev, head_rev);
	}

	// 使用 p4 changes -m 1 获取 LatestUser
	let changes_output = Command::new("p4")
		.arg("changes")
		.arg("-m")
		.arg("1")
		.arg(path)
		.output();

	if let Ok(out) = changes_output {
		let stdout = String::from_utf8_lossy(&out.stdout);
		if let Some(line) = stdout.lines().next() {
			let parts: Vec<&str> = line.split_whitespace().collect();
			if parts.len() >= 6 {
				detail.latest_user = parts[5].split('@').next().unwrap_or(parts[5]).to_string();
			}
		}
	}

	save_to_cache(path, &detail);

	Ok(detail)
}

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
	let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
	clipboard.set_text(text.to_owned()).map_err(|e| e.to_string())?;
	Ok(())
}

fn format_size(bytes: u64) -> String {
	if bytes < 1024 {
		format!("{} B", bytes)
	} else if bytes < 1024 * 1024 {
		format!("{:.1} KB", bytes as f64 / 1024.0)
	} else {
		format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
	}
}

fn format_time(timestamp: i64) -> String {
	if timestamp == 0 { return "".to_string(); }
	let dt = Local.timestamp_opt(timestamp, 0).unwrap();
	dt.format("%m/%d %H:%M").to_string()
}

fn save_to_cache(path: &Path, detail: &FileDetail) {
	if let Some(cache_dir) = dirs::cache_dir() {
		let pier_cache = cache_dir.join("pier").join("details");
		let _ = fs::create_dir_all(&pier_cache);
		
		let hash = md5_hash(path.to_string_lossy().as_ref());
		let cache_file = pier_cache.join(format!("{}.json", hash));
		if let Ok(json) = serde_json::to_string_pretty(detail) {
			let _ = fs::write(cache_file, json);
		}
	}
}

fn load_from_cache(path: &Path) -> Option<FileDetail> {
	if let Some(cache_dir) = dirs::cache_dir() {
		let pier_cache = cache_dir.join("pier").join("details");
		let hash = md5_hash(path.to_string_lossy().as_ref());
		let cache_file = pier_cache.join(format!("{}.json", hash));
		if let Ok(data) = fs::read_to_string(cache_file) {
			return serde_json::from_str(&data).ok();
		}
	}
	None
}

fn md5_hash(s: &str) -> String {
	s.replace('/', "_")
}
