/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::config::REPART_DEFINITIONS_DIR;
use futures_util::future::{FutureExt, RemoteHandle};
use futures_util::{StreamExt, TryStreamExt};
use log::error;
use tokio::pin;
use tokio::runtime::Builder;

mod systemd_repart;
mod systemd_repart_demo;
pub use systemd_repart::{CandidateDevice, ProgressPhase};
use systemd_repart::{EmptyMode, Repart as RepartTrait, RepartChain, RepartError, RunOutput};
use systemd_repart_demo::RepartDemo;

use crate::models::{Disk, DiskStatus, InstallPhase};

pub const SOCKET_PATH: &str = "/run/systemd/io.systemd.Repart";

pub struct Repart {
    handle: tokio::runtime::Handle,
    #[allow(dead_code)]
    pending_remote_handle: RemoteHandle<()>,
}

impl Repart {
    pub fn new() -> Self {
        // FIXME: zlink currently only supports tokio, would be nice to spawn tasks directly on the glib main loop
        let rt = Builder::new_current_thread()
            .enable_io()
            .build()
            .expect("Unable to create tokio runtime");

        let handle = rt.handle().clone();
        let (pending, pending_remote_handle) = std::future::pending().remote_handle();

        // Create a thread to run the tokio scheduler and eventually spawned tasks
        // Dropping pending_remote_handle will abort the pending future and then terminate the thread
        std::thread::spawn(move || {
            rt.block_on(pending);
        });

        Self {
            handle,
            pending_remote_handle,
        }
    }

    pub async fn candidate_devices(&self) -> zlink::Result<Vec<Disk>> {
        let _guard = self.handle.enter();

        let mut demo_server = if crate::is_demo() {
            Some(RepartDemo::run().await)
        } else {
            None
        };

        let connection = if let Some(ref mut demo_server) = demo_server {
            demo_server.connection()
        } else {
            &mut zlink::unix::connect(SOCKET_PATH).await?
        };

        let disks: Vec<Disk> = connection
            .list_candidate_devices(None, None)
            .await?
            .filter_map(|res| async move {
                if let Ok(Ok(device)) = res {
                    Some(Disk::from_candidate_device(&device))
                } else {
                    None
                }
            })
            .collect()
            .await;

        let mut chain = connection.chain_run(
            None,                      // node
            &EmptyMode::Force,         // empty_mode. Use force for erease
            true,                      // dry_run
            &[REPART_DEFINITIONS_DIR], // definitions
            None,                      // defer_partitions_empty
        )?;

        for disk in disks.iter() {
            let node = disk.node();
            chain = chain.run(
                Some(&node),               // node
                &EmptyMode::Allow,         // empty_mode. Use force for erease
                true,                      // dry_run
                &[REPART_DEFINITIONS_DIR], // definitions
                None,                      // defer_partitions_empty
            )?;
        }

        let replies = chain.send::<RunOutput, RepartError>().await?;
        pin!(replies);
        // The first reply is without any node.
        // FIXME: Repart doesn't report the correct minimal disk size
        // See: https://github.com/systemd/systemd/issues/39788
        let minimal_size_bytes = replies
            .try_next()
            .await?
            .and_then(|(reply, _)| reply.ok())
            .and_then(|reply| reply.into_parameters())
            .and_then(|reply| reply.minimal_size_bytes)
            .unwrap_or_default();

        let mut disk = disks.iter();
        while let Some((reply_allow, _)) = replies.try_next().await? {
            let disk = disk.next().expect("Expected a response for each device");

            disk.set_min_disk_size(minimal_size_bytes);

            match reply_allow {
                Err(RepartError::ConflictingDiskLabelPresent) => {
                    disk.set_status(DiskStatus::UnsupportedPartitionTable);
                }
                Err(RepartError::InsufficientFreeSpace {
                    current_size_bytes, ..
                }) => {
                    if current_size_bytes == Some(disk.size()) {
                        disk.set_status(DiskStatus::Full);
                    } else {
                        disk.set_status(DiskStatus::InsufficientFreeSpace);
                    }
                }
                Err(RepartError::DiskTooSmall { .. }) => {
                    disk.set_status(DiskStatus::TooSmall);
                }
                // Not a single candidate block device could be found.
                Err(RepartError::NoCandidateDevices) => {
                    unreachable!("The systemd-repart run method never returns RepartError::NoCandidateDevices");
                }
                Ok(output) => {
                    if let Some(RunOutput {
                        current_size_bytes, ..
                    }) = output.into_parameters()
                    {
                        let status = if current_size_bytes == Some(0) {
                            DiskStatus::Empty
                        } else {
                            DiskStatus::DataDetected
                        };
                        disk.set_status(status);
                    }
                }
            }
        }

        Ok(disks)
    }

    pub async fn install(&self, disk: &Disk, erase: bool) -> zlink::Result<()> {
        let _guard = self.handle.enter();

        let mut demo_server = if crate::is_demo() {
            Some(RepartDemo::run().await)
        } else {
            None
        };

        let connection = if let Some(ref mut demo_server) = demo_server {
            demo_server.connection()
        } else {
            &mut zlink::unix::connect(SOCKET_PATH).await?
        };

        let empy_mode = if erase {
            EmptyMode::Force
        } else {
            EmptyMode::Allow
        };
        let node = disk.node();

        let stream = connection
            .run_with_progress(
                Some(&node),               // node
                &empy_mode,                // empty_mode
                false,                     // dry_run
                &[REPART_DEFINITIONS_DIR], // definitions
                Some(true),                // defer_partitions_empty
            )
            .await?;

        tokio::pin!(stream);
        while let Some(progress) = stream.try_next().await? {
            if let Ok(progress) = progress {
                let phase: InstallPhase = progress.phase.into();
                disk.set_install_phase(phase);
                disk.set_install_progress(progress.progress.unwrap_or_default());
            } else {
                error!("Received unexpected error during installation");
            }
        }

        disk.set_install_phase(InstallPhase::Finished);
        disk.set_install_progress(100);

        Ok(())
    }
}

impl From<Option<ProgressPhase>> for InstallPhase {
    fn from(phase: Option<ProgressPhase>) -> Self {
        let Some(phase) = phase else {
            return InstallPhase::None;
        };
        match phase {
            ProgressPhase::LoadingDefinitions
            | ProgressPhase::LoadingTable
            | ProgressPhase::OpeningCopyBlockSources
            | ProgressPhase::AcquiringPartitionLabels => InstallPhase::WarmUp,
            ProgressPhase::Minimizing => InstallPhase::Minimizing,
            ProgressPhase::Placing => InstallPhase::Placing,
            ProgressPhase::WipingDisk => InstallPhase::WipingDisk,
            ProgressPhase::WipingPartition => InstallPhase::WipingPartition,
            ProgressPhase::CopyingPartition => InstallPhase::CopyingPartition,
            ProgressPhase::FormattingPartition => InstallPhase::FormattingPartition,
            ProgressPhase::AdjustingPartition => InstallPhase::AdjustingPartition,
            ProgressPhase::WritingTable | ProgressPhase::RereadingTable => {
                InstallPhase::WritingTable
            }
        }
    }
}
