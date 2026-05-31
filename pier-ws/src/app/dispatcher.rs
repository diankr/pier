use crate::app::App;
use crate::app::sync::SyncEvent;
use pier_core::filetree::FileP4Status;
use anyhow::Result;

pub(crate) struct Dispatcher<'a> {
	pub(crate) app: &'a mut App,
}

impl<'a> Dispatcher<'a> {
	pub(crate) fn new(app: &'a mut App) -> Self {
		Self { app }
	}

	pub(crate) fn process_events(&mut self) -> Result<()> {
		self.process_detail()?;
		self.process_status()?;
		self.process_sync()?;
		Ok(())
	}

	fn process_detail(&mut self) -> Result<()> {
		while let Ok(result) = self.app.detail_rx.try_recv() {
			match result {
				Ok(detail) => {
					self.app.core.current_detail = Some(detail);
					self.app.core.detail_error = None;
				}
				Err(e) => {
					self.app.core.current_detail = None;
					self.app.core.detail_error = Some(e);
				}
			}
		}
		Ok(())
	}

	fn process_status(&mut self) -> Result<()> {
		while let Ok(statuses) = self.app.status_rx.try_recv() {
			for file in self.app.core.filetree.files.iter_mut().chain(self.app.core.filetree.parent_files.iter_mut()) {
				if file.is_dir { continue; }
				if let Some(status) = statuses.get(&file.path) {
					file.p4_status = status.clone();
				} else {
					file.p4_status = FileP4Status::Untracked;
				}
			}
		}
		Ok(())
	}

	fn process_sync(&mut self) -> Result<()> {
		while let Ok(event) = self.app.sync_rx.try_recv() {
			match event {
				SyncEvent::Start(files) => {
					self.app.core.sync_total_bytes = files.iter().map(|f| f.size).sum();
					self.app.core.sync_files = files;
					self.app.core.sync_total = self.app.core.sync_files.len();
					self.app.core.sync_current = 0;
					self.app.core.sync_synced_bytes = 0;
					self.app.core.sync_progress = 0.0;
				}
				SyncEvent::FileDone(_file) => {
					self.app.core.sync_current += 1;
				}
				SyncEvent::ByteProgress(progress_vec) => {
					self.app.core.sync_synced_bytes = progress_vec.iter().sum();
					for file in self.app.core.sync_files.iter_mut() {
						if let Some(&synced) = progress_vec.get(file.original_index) {
							file.synced = synced;
						}
					}
					
					self.app.core.sync_files.sort_by(|a, b| {
						let a_started = a.synced > 0;
						let b_started = b.synced > 0;
						b_started.cmp(&a_started)
					});

					if self.app.core.sync_total_bytes > 0 {
						self.app.core.sync_progress = self.app.core.sync_synced_bytes as f64 / self.app.core.sync_total_bytes as f64;
					}
				}
				SyncEvent::End => {
					self.app.core.is_syncing = false;
					self.app.core.detect_synced_change();
					self.app.sync_handle = None;
					self.app.sync_cancel_tx = None;
				}
				SyncEvent::Error(e) => {
					self.app.core.is_syncing = false;
					self.app.core.add_log("Sync Error", &e);
					self.app.sync_handle = None;
					self.app.sync_cancel_tx = None;
				}
			}
		}
		Ok(())
	}
}
