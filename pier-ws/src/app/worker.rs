use std::path::PathBuf;
use pier_core::detail::FileDetail;
use pier_core::filetree::FileP4Status;
use std::collections::HashMap;

pub(crate) fn spawn_detail_worker(
	mut rx: tokio::sync::mpsc::UnboundedReceiver<PathBuf>,
	tx: tokio::sync::mpsc::UnboundedSender<anyhow::Result<FileDetail, String>>,
) {
	tokio::spawn(async move {
		while let Some(path) = rx.recv().await {
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
			if rx.len() > 0 {
				continue;
			}
			let result = pier_core::detail::fetch_file_detail(&path);
			let _ = tx.send(result);
		}
	});
}

pub(crate) fn spawn_status_worker(
	mut rx: tokio::sync::mpsc::UnboundedReceiver<Vec<PathBuf>>,
	tx: tokio::sync::mpsc::UnboundedSender<HashMap<PathBuf, FileP4Status>>,
) {
	tokio::spawn(async move {
		while let Some(paths) = rx.recv().await {
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
			if rx.len() > 0 {
				continue;
			}
			let statuses = pier_core::core::fetch_file_statuses(&paths);
			let _ = tx.send(statuses);
		}
	});
}
