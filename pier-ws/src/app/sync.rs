use std::mem;

pub enum SyncEvent {
	Start(Vec<pier_core::core::SyncFileInfo>),
	FileDone(String),
	End,
	Error(String),
}

pub(crate) fn parse_ztag_sync(output: &str) -> Vec<pier_core::core::SyncFileInfo> {
	let mut files = Vec::new();
	let mut depot_path = String::new();
	let mut local_path = String::new();

	for line in output.lines() {
		if let Some(rest) = line.strip_prefix("... depotFile ") {
			if !depot_path.is_empty() {
				files.push(pier_core::core::SyncFileInfo {
					depot_path: mem::take(&mut depot_path),
					local_path: mem::take(&mut local_path),
					status: pier_core::core::SyncFileStatus::Pending,
					original_index: files.len(),
				});
			}
			depot_path = rest.to_string();
		} else if let Some(rest) = line.strip_prefix("... clientFile ") {
			local_path = rest.to_string();
		}
	}

	if !depot_path.is_empty() {
		files.push(pier_core::core::SyncFileInfo {
			depot_path,
			local_path,
			status: pier_core::core::SyncFileStatus::Pending,
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

		let _ = tx.send(SyncEvent::Start(files.clone()));

		// 2. Actual sync - individual files for granular progress
		for file in files {
			tokio::select! {
				_ = &mut cancel_rx => {
					return;
				}
				_ = async {
					let mut cmd = tokio::process::Command::new("p4");
					let path_with_rev = if let Some(id) = &cl_id {
						format!("{}@{}", file.depot_path, id)
					} else {
						file.depot_path.clone()
					};
					
					cmd.arg("sync").arg(path_with_rev);
					
					// We don't need piped output here since we know which file we're syncing
					let _ = cmd.output().await;
					let _ = tx.send(SyncEvent::FileDone(file.depot_path.clone()));
				} => {}
			}
		}

		let _ = tx.send(SyncEvent::End);
	})
}
