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
				SyncEvent::Start(mut files) => {
					if let Some(first) = files.get_mut(0) {
						first.status = pier_core::core::SyncFileStatus::Syncing;
					}
					self.app.core.sync_files = files;
					self.app.core.sync_total = self.app.core.sync_files.len();
					self.app.core.sync_current = 0;
					self.app.core.sync_progress = 0.0;
				}
				SyncEvent::FileDone(path) => {
					// Structured matching: path is now likely the depot path from -ztag
					let mut found_index = None;
					for (i, file) in self.app.core.sync_files.iter().enumerate() {
						if file.depot_path == path || path.contains(&file.depot_path) || (!file.local_path.is_empty() && path.contains(&file.local_path)) {
							found_index = Some(i);
							break;
						}
					}

					if let Some(idx) = found_index {
						if self.app.core.sync_files[idx].status != pier_core::core::SyncFileStatus::Done {
							self.app.core.sync_files[idx].status = pier_core::core::SyncFileStatus::Done;
							self.app.core.sync_current += 1;
						}
					} else {
						// Fallback: mark the first 'Syncing' file as Done if we can't match specifically
						if let Some(idx) = self.app.core.sync_files.iter().position(|f| f.status == pier_core::core::SyncFileStatus::Syncing) {
							self.app.core.sync_files[idx].status = pier_core::core::SyncFileStatus::Done;
							self.app.core.sync_current += 1;
						}
					}

					// Ensure exactly one file is 'Syncing' (the first Pending one)
					for file in self.app.core.sync_files.iter_mut() {
						if file.status == pier_core::core::SyncFileStatus::Syncing {
							file.status = pier_core::core::SyncFileStatus::Syncing; // stay syncing
						}
					}
					
					// If no file is currently syncing, pick the next pending one
					if !self.app.core.sync_files.iter().any(|f| f.status == pier_core::core::SyncFileStatus::Syncing) {
						if let Some(next_idx) = self.app.core.sync_files.iter().position(|f| f.status == pier_core::core::SyncFileStatus::Pending) {
							self.app.core.sync_files[next_idx].status = pier_core::core::SyncFileStatus::Syncing;
						}
					}

					if self.app.core.sync_total > 0 {
						self.app.core.sync_progress = (self.app.core.sync_current as f64 / self.app.core.sync_total as f64).min(1.0);
					}

					// Sort to put Done/Syncing files at top
					self.app.core.sync_files.sort_by(|a, b| {
						use pier_core::core::SyncFileStatus::*;
						match (&a.status, &b.status) {
							(Syncing, _) => std::cmp::Ordering::Less,
							(_, Syncing) => std::cmp::Ordering::Greater,
							(Done, Done) => b.original_index.cmp(&a.original_index),
							(Done, Pending) => std::cmp::Ordering::Less,
							(Pending, Done) => std::cmp::Ordering::Greater,
							(Pending, Pending) => a.original_index.cmp(&b.original_index),
						}
					});
				}
				SyncEvent::End => {
					// Mark all as done on exit to ensure 100% progress
					for file in self.app.core.sync_files.iter_mut() {
						file.status = pier_core::core::SyncFileStatus::Done;
					}
					self.app.core.sync_current = self.app.core.sync_total;
					self.app.core.sync_progress = 1.0;
					self.app.core.sync_finished = true; // Set flag
					
					// Sort one last time
					self.app.core.sync_files.sort_by(|a, b| {
						b.original_index.cmp(&a.original_index)
					});

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
