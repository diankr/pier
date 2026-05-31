use tokio::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::time::interval;
use std::fs;
use std::mem;

pub enum SyncEvent {
	Start(Vec<pier_core::core::SyncFileInfo>),
	FileDone(String),
	ByteProgress(Vec<u64>),
	End,
	Error(String),
}

pub(crate) fn parse_ztag_sync(output: &str) -> Vec<pier_core::core::SyncFileInfo> {
	let mut files = Vec::new();
	let mut depot_path = String::new();
	let mut local_path = String::new();
	let mut size = 0;

	for line in output.lines() {
		if let Some(rest) = line.strip_prefix("... depotFile ") {
			if !depot_path.is_empty() {
				files.push(pier_core::core::SyncFileInfo {
					depot_path: mem::take(&mut depot_path),
					local_path: mem::take(&mut local_path),
					size,
					synced: 0,
					original_index: files.len(),
				});
				size = 0;
			}
			depot_path = rest.to_string();
		} else if let Some(rest) = line.strip_prefix("... clientFile ") {
			local_path = rest.to_string();
		} else if let Some(rest) = line.strip_prefix("... fileSize ") {
			size = rest.parse().unwrap_or(0);
		}
	}

	if !depot_path.is_empty() {
		files.push(pier_core::core::SyncFileInfo {
			depot_path,
			local_path,
			size,
			synced: 0,
			original_index: files.len(),
		});
	}

	files
}

pub(crate) fn spawn_sync_task(
	cl_id: Option<String>,
	virtual_root: Option<std::path::PathBuf>,
	tx: tokio::sync::mpsc::UnboundedSender<SyncEvent>,
	mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
	tokio::spawn(async move {
		// 1. Dry run to get files with ztag
		let mut cmd = tokio::process::Command::new("p4");
		cmd.arg("-ztag").arg("sync").arg("-n");
		if let Some(id) = &cl_id {
			cmd.arg(format!("@{}", id));
		}
		
		if let Some(ref vr) = virtual_root {
			let vr_str = vr.to_string_lossy();
			cmd.arg(format!("{}/...", vr_str));
		}
		
		let output = cmd.output().await;
		let files = if let Ok(out) = output {
			parse_ztag_sync(&String::from_utf8_lossy(&out.stdout))
		} else {
			vec![]
		};

		if files.is_empty() {
			let _ = tx.send(SyncEvent::End);
			return;
		}

		// Local paths for progress tracking
		let sync_info: Vec<(String, u64)> = files.iter().map(|f| (f.local_path.clone(), f.size)).collect();
		
		let _ = tx.send(SyncEvent::Start(files));

		// 2. Actual sync
		let mut cmd = tokio::process::Command::new("p4");
		cmd.arg("sync");
		if let Some(id) = cl_id {
			cmd.arg(format!("@{}", id));
		}

		if let Some(ref vr) = virtual_root {
			let vr_str = vr.to_string_lossy();
			cmd.arg(format!("{}/...", vr_str));
		}

		cmd.stdout(std::process::Stdio::piped());
		cmd.stderr(std::process::Stdio::piped());
		
		let mut child = match cmd.spawn() {
			Ok(c) => c,
			Err(e) => {
				let _ = tx.send(SyncEvent::Error(e.to_string()));
				return;
			}
		};

		let stdout = child.stdout.take().unwrap();
		let mut reader = tokio::io::BufReader::new(stdout);
		
		let (done_tx, mut done_rx) = tokio::sync::oneshot::channel::<()>();
		let tx_progress = tx.clone();
		
		// Progress monitor task
		tokio::spawn(async move {
			let mut interval = interval(Duration::from_millis(100));
			loop {
				tokio::select! {
					_ = &mut done_rx => break,
					_ = interval.tick() => {
						let mut progress_vec = Vec::with_capacity(sync_info.len());
						for (path, expected_size) in &sync_info {
							let synced = if let Ok(metadata) = fs::metadata(path) {
								metadata.len().min(*expected_size)
							} else {
								0
							};
							progress_vec.push(synced);
						}
						let _ = tx_progress.send(SyncEvent::ByteProgress(progress_vec));
					}
				}
			}
		});

		loop {
			let mut line = String::new();
			tokio::select! {
				_ = &mut cancel_rx => {
					let _ = child.kill().await;
					let _ = done_tx.send(());
					return;
				}
				res = reader.read_line(&mut line) => {
					match res {
						Ok(0) => break,
						Ok(_) => {
							let trimmed = line.trim();
							if !trimmed.is_empty() {
								let _ = tx.send(SyncEvent::FileDone(trimmed.to_string()));
							}
						}
						Err(_) => break,
					}
				}
			}
		}

		let _ = child.wait().await;
		let _ = done_tx.send(());
		let _ = tx.send(SyncEvent::End);
	})
}
