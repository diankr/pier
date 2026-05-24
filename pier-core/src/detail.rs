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
		.arg(path)
		.output()
		.map_err(|e| format!("Failed to execute p4 fstat: {}", e))?;

	if !output.status.success() {
		return Err("Not a Perforce-managed object".to_string());
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let mut detail = FileDetail {
		filename: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
		filesize: "Unknown".to_string(),
		depot_path: "Unknown".to_string(),
		revision: "Unknown".to_string(),
		date_modified: "Unknown".to_string(),
		changelist: "Unknown".to_string(),
		action: "Unknown".to_string(),
		latest_user: "Unknown".to_string(),
		checkout_by: "Unknown".to_string(),
	};

	for line in stdout.lines() {
		let parts: Vec<&str> = line.splitn(3, ' ').collect();
		if parts.len() < 3 { continue; }
		let key = parts[1];
		let value = parts[2];

		match key {
			"depotFile" => detail.depot_path = value.to_string(),
			"headRev" => {
				let have_rev = detail.revision.split('/').next().unwrap_or("0");
				detail.revision = format!("{}/{}", have_rev, value);
			}
			"haveRev" => {
				let head_rev = detail.revision.split('/').nth(1).unwrap_or("0");
				detail.revision = format!("{}/{}", value, head_rev);
			}
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
			"otherOpen" => detail.checkout_by = "Other Users".to_string(),
			"action" => detail.checkout_by = "You".to_string(),
			_ => {}
		}
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

fn format_size(bytes: u64) -> String {
	if bytes < 1024 {
		format!("{} B", bytes)
	} else if bytes < 1024 * 1024 {
		format!("{:.1} KB", bytes as f64 / 1024.0)
	} else {
		format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
	}
}

fn format_time(_timestamp: i64) -> String {
	// 使用 std 暂难完美实现，此处暂用简单格式，后续可引入 chrono
	// 这里我们暂且返回原值或简化处理
	"05/25 10:00".to_string() 
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

fn md5_hash(s: &str) -> String {
	// 简化处理，暂时直接用字符串，后续可引入 md5 crate
	s.replace('/', "_")
}
