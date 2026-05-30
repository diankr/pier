use std::process::Command;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::fs;

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
	// Debug log
	// println!("fstat output for {}: {}", path.display(), stdout);

	let mut detail = FileDetail {
		filename: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
		filesize: "".to_string(),
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
		
		let content = trimmed.trim_start_matches('.').trim();
		let mut split = content.splitn(2, ' ');
		let key = split.next().unwrap_or("");
		let value = split.next().unwrap_or("").trim();

		match key {
			k if k.eq_ignore_ascii_case("depotFile") => {
				if let Some(last_slash) = value.rfind('/') {
					let dir = &value[..last_slash];
					if dir.starts_with("//depot") {
						detail.depot_path = dir.replacen("//depot", "...", 1);
					} else {
						detail.depot_path = dir.to_string();
					}
					if !detail.depot_path.ends_with('/') {
						detail.depot_path.push('/');
					}
				}
			}
			k if k.eq_ignore_ascii_case("headRev") => head_rev = value.trim_start_matches('#').to_string(),
			k if k.eq_ignore_ascii_case("haveRev") => have_rev = value.trim_start_matches('#').to_string(),
			k if k.eq_ignore_ascii_case("fileSize") => {
				let size_bytes: u64 = value.parse().unwrap_or(0);
				if size_bytes > 0 {
					detail.filesize = format_size(size_bytes);
				}
			}
			k if k.eq_ignore_ascii_case("headChange") => detail.changelist = value.to_string(),
			k if k.to_lowercase().starts_with("otheropen") => {
				// common keys: otherOpen (count), otherOpen0, otherOpen1...
				// Skip "otherOpen" count and only take values with @ (user@client)
				if k.len() > 9 && value.contains('@') {
					if detail.checkout_by.is_empty() || detail.checkout_by == "Unknown" {
						detail.checkout_by = value.split('@').next().unwrap_or(value).to_string();
					}
				}
			}
			k if k.eq_ignore_ascii_case("action") || k.eq_ignore_ascii_case("change") => {
				if detail.checkout_by.is_empty() || detail.checkout_by == "Unknown" {
					detail.checkout_by = "You".to_string();
				}
				if k.eq_ignore_ascii_case("action") {
					detail.action = value.to_string();
				}
			}
			k if k.eq_ignore_ascii_case("headAction") => {
				if detail.action.is_empty() {
					detail.action = value.to_string();
				}
			}
			_ => {}
		}
	}
	
	// Revision 格式化: 始终显示为 #have/head 格式 (例如 #2/2)
	let have_n: u32 = have_rev.parse().unwrap_or(0);
	let mut head_n: u32 = head_rev.parse().unwrap_or(0);

	// 安全检查：由于 P4 fstat 在某些特殊映射下可能返回较小的 headRev，
	// 我们确保总版本号至少等于当前拥有的版本号。
	if head_n < have_n {
		head_n = have_n;
	}

	// 始终显示 #have/head
	detail.revision = format!("#{}/{}", have_n, head_n);

	// 如果 fileSize 还是空的，尝试本地文件大小或 p4 sizes
	if detail.filesize.is_empty() {
		if let Ok(meta) = fs::metadata(path) {
			let size = meta.len();
			if size > 0 {
				detail.filesize = format_size(size);
			}
		}
	}

	if detail.filesize.is_empty() {
		let sizes_output = Command::new("p4")
			.arg("sizes")
			.arg(path)
			.output();
		if let Ok(out) = sizes_output {
			let stdout = String::from_utf8_lossy(&out.stdout);
			// p4 sizes 输出可能包含多个文件，取第一个
			if let Some(line) = stdout.lines().next() {
				let parts: Vec<&str> = line.split_whitespace().collect();
				// 格式通常是: //path#rev [count blocks] size bytes
				for (i, part) in parts.iter().enumerate() {
					if *part == "bytes" && i > 0 {
						if let Ok(size) = parts[i-1].parse::<u64>() {
							detail.filesize = format_size(size);
							break;
						}
					}
				}
			}
		}
	}
	if detail.filesize.is_empty() {
		detail.filesize = "0 B".to_string();
	}

	// 使用 p4 changes -m 1 -t 获取 LatestUser 和 DateModified
	let changes_output = Command::new("p4")
		.arg("changes")
		.arg("-m")
		.arg("1")
		.arg("-t")
		.arg(path)
		.output();

	if let Ok(out) = changes_output {
		let stdout = String::from_utf8_lossy(&out.stdout);
		if let Some(line) = stdout.lines().next() {
			let parts: Vec<&str> = line.split_whitespace().collect();
			if parts.len() >= 7 {
				let date = parts[3]; // 2023/05/11
				let time_full = parts[4]; // 16:33:14
				let author_full = parts[6]; // user@workspace

				detail.latest_user = author_full.split('@').next().unwrap_or(author_full).to_string();
				
				let date_parts: Vec<&str> = date.split('/').collect();
				let formatted_date = if date_parts.len() == 3 {
					format!("{}/{}", date_parts[1], date_parts[2])
				} else {
					date.to_string()
				};

				let time_parts: Vec<&str> = time_full.split(':').collect();
				let formatted_time = if time_parts.len() >= 2 {
					format!("{}:{}", time_parts[0], time_parts[1])
				} else {
					time_full.to_string()
				};

				detail.date_modified = format!("{} {}", formatted_date, formatted_time);
			}
		}
	}

	save_to_cache(path, &detail);

	// Fallback for checkout_by using p4 opened -a if it's still empty or "Unknown"
	if detail.checkout_by.is_empty() || detail.checkout_by == "Unknown" {
		let opened_output = Command::new("p4")
			.arg("opened")
			.arg("-a")
			.arg(path)
			.output();
		
		if let Ok(out) = opened_output {
			let stdout = String::from_utf8_lossy(&out.stdout);
			if let Some(line) = stdout.lines().next() {
				// line format: //depot/path#rev - action change user@client (type)
				if let Some(dash_idx) = line.find(" - ") {
					let after_dash = &line[dash_idx + 3..];
					let parts: Vec<&str> = after_dash.split_whitespace().collect();
					
					// Find the part that contains '@', which should be user@client
					if let Some(user_at_client) = parts.iter().find(|p| p.contains('@')) {
						let user = user_at_client.split('@').next().unwrap_or(user_at_client);
						
						// Check if it's the current user/client
						let p4_user = std::env::var("P4USER").unwrap_or_default();
						let p4_client = std::env::var("P4CLIENT").unwrap_or_default();
						if !p4_user.is_empty() && !p4_client.is_empty() && user_at_client.contains(&format!("{}@{}", p4_user, p4_client)) {
							detail.checkout_by = "You".to_string();
						} else {
							detail.checkout_by = user.to_string();
						}
						save_to_cache(path, &detail);
					}
				}
			}
		}
	}

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

pub fn load_from_cache(path: &Path) -> Option<FileDetail> {
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
	format!("v5_{}", s.replace('/', "_"))
}
