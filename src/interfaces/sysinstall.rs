/* SPDX-License-Identifier: GPL-3.0-or-later */

mod systemd_sysinstall;
mod systemd_sysinstall_demo;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::LazyLock;

use systemd_sysinstall_demo::SysInstallDemo;

use crate::config::REPART_DEFINITIONS_DIR;
use log::error;
use zlink::futures_util::StreamExt;

use crate::models::disk::{Disk, DiskStatus, InstallPhase};

use systemd_sysinstall::{
    BlockDeviceAction, CandidateDevice, DeviceFit, ProgressPhase, SysInstall as SysInstallTrait,
};

const SOCKET_PATH: &str = "/run/systemd/io.systemd.SysInstall";

static KERNEL_IAMGE_PATH: LazyLock<Option<String>> = LazyLock::new(|| {
    let file = match File::open("/etc/os-release") {
        Ok(file) => file,
        Err(error) => {
            error!("Failed to read /etc/os-release: {error}");
            return None;
        }
    };
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    loop {
        let line = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(error)) => {
                error!("Failed to read IMAGE_VERSION from /etc/os-release: {error}");
                return None;
            }
            None => {
                error!("Can't find IMAGE_VERSION in /etc/os-release");
                return None;
            }
        };

        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap().trim();
        let value = match parts.next() {
            Some(v) => v.trim(),
            None => continue,
        };

        if key == "IMAGE_VERSION" {
            let image_version = value.trim_matches(['"', '\'']);
            let kernel_image =
                format!("/var/lib/gnomeos/installer-esp/EFI/Linux/gnomeos_{image_version}.efi");

            log::debug!("Use kernel image at {kernel_image}");

            return Some(kernel_image);
        }
    }
});

impl Disk {
    pub fn from_candidate_device(device: CandidateDevice) -> Option<Self> {
        let CandidateDevice {
            node,
            subsystem,
            vendor,
            model,
            size_bytes,
            fit,
            minimal_size_bytes,
            current_size_bytes,
            ..
        } = device;

        // Make sure this is a actual node
        if node.is_none() || fit.is_none() {
            return None;
        }

        let status = match fit.unwrap() {
            DeviceFit::ConflictingDiskLabelPresent => DiskStatus::UnsupportedPartitionTable,
            DeviceFit::InsufficientFreeSpace => DiskStatus::InsufficientFreeSpace,
            DeviceFit::DiskTooSmall => DiskStatus::TooSmall,
            DeviceFit::EnoughFreeSpace => {
                if current_size_bytes == Some(0i64) {
                    DiskStatus::Empty
                } else {
                    DiskStatus::DataDetected
                }
            }
        };

        Some(
            glib::Object::builder()
                .property("node", node.unwrap())
                .property("subsystem", subsystem.as_ref().filter(|s| !s.is_empty()))
                .property("vendor", vendor.as_ref().filter(|s| !s.is_empty()))
                .property("model", model.as_ref().filter(|s| !s.is_empty()))
                .property("size", size_bytes.unwrap_or_default())
                .property("min-disk-size", minimal_size_bytes.unwrap_or_default())
                .property("status", status)
                .build(),
        )
    }
}

pub struct SysInstall {}

impl SysInstall {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn candidate_devices(
        &self,
        add: impl Fn(Disk),
        remove: impl Fn(&str),
    ) -> zlink::Result<()> {
        let mut demo_server = if crate::is_demo() {
            Some(SysInstallDemo::run().await)
        } else {
            None
        };

        let connection = if let Some(ref mut demo_server) = demo_server {
            demo_server.connection()
        } else {
            &mut zlink::unix::connect(SOCKET_PATH).await?
        };

        let mut stream =
            connection.list_candidate_devices(&[REPART_DEFINITIONS_DIR], Some(true)).await?;

        while let Some(result) = stream.next().await {
            let reply = match result {
                Ok(Ok(reply)) => reply,
                Ok(Err(error)) => {
                    error!("Unexpected response while fetching candidate devices:  {error:?}");
                    continue;
                }
                Err(error) => {
                    error!("Connection to sysinstall failed: {error:?}");
                    continue;
                }
            };

            let Some(action) = reply.action.as_ref() else {
                if let Some(disk) = Disk::from_candidate_device(reply) {
                    add(disk);
                }
                continue;
            };

            match action {
                BlockDeviceAction::Ready => {}
                BlockDeviceAction::Add => {
                    if let Some(disk) = Disk::from_candidate_device(reply) {
                        add(disk);
                    }
                }
                BlockDeviceAction::Remove => {
                    if let Some(node) = reply.node {
                        remove(&node);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn install(&self, disk: &Disk, erase: bool) -> zlink::Result<()> {
        let mut demo_server = if crate::is_demo() {
            Some(SysInstallDemo::run().await)
        } else {
            None
        };

        let connection = if let Some(ref mut demo_server) = demo_server {
            demo_server.connection()
        } else {
            &mut zlink::unix::connect(SOCKET_PATH).await?
        };

        let node = disk.node();

        let mut stream = connection
            .run_with_progress(
                &node,                        // node
                &[REPART_DEFINITIONS_DIR],    // definitions
                erase,                        // erase
                KERNEL_IAMGE_PATH.as_deref(), // kernel_image
                true,                         // variables
                Some(true),                   // copy_locale
                Some(true),                   // copy_timezone
                Some(true),                   // copy_keymap
            )
            .await?;

        while let Some(result) = stream.next().await {
            match result {
                Ok(progress) => {
                    if let Ok(progress) = progress {
                        let phase: InstallPhase = progress.phase.into();
                        disk.set_install_phase(phase);
                        // Only update the progress if sysinstall reported progress
                        if let Some(progress) = progress.progress {
                            disk.set_install_progress(progress);
                        }
                    } else {
                        error!("Received unexpected error during installation");
                    }
                }
                Err(zlink::Error::MissingParameters) => {
                    // SysInstall sends a empty reply once the installation is
                    // finished and zlink reports it as an error, so ignore it.
                }
                Err(error) => {
                    error!("Received unexpected error during installation: {error}");
                }
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
            ProgressPhase::ValidateBlockDevice
            | ProgressPhase::LoadCredentials
            | ProgressPhase::EncryptCredentials => InstallPhase::WarmUp,
            ProgressPhase::InstallPartitions => InstallPhase::CopyingPartition,
            ProgressPhase::MountPartitions
            | ProgressPhase::InstallKernel
            | ProgressPhase::InstallBootloader
            | ProgressPhase::UnmountPartitions => InstallPhase::InstallBootloader,
        }
    }
}
