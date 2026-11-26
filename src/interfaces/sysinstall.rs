/* SPDX-License-Identifier: GPL-3.0-or-later */

mod systemd_sysinstall;
mod systemd_sysinstall_demo;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{LazyLock, Mutex, OnceLock};

use anyhow::anyhow;

use systemd_sysinstall_demo::SysInstallDemo;

use crate::config::REPART_DEFINITIONS_DIR;
use log::error;
use zlink::futures_util::StreamExt;

use crate::models::disk::{Disk, DiskStatus, InstallPhase};

use systemd_sysinstall::{
    BlockDeviceAction, CandidateDevice, DeviceFit, ProgressPhase, SysInstall as SysInstallTrait,
};

const DEFAULT_SOCKET_PATH: &str = "/run/systemd/io.systemd.SysInstall";

static SOCKET_PATH: OnceLock<String> = OnceLock::new();

pub fn set_sysinstall_socket(path: Option<String>) {
    if let Some(path) = path.filter(|path| !path.is_empty()) {
        let _ = SOCKET_PATH.set(path);
    }
}

fn socket_path() -> &'static str {
    SOCKET_PATH.get().map_or(DEFAULT_SOCKET_PATH, String::as_str)
}

/// Unit the distribution wants started (with the target disk as its instance) after
/// systemd-sysinstall copied the OS, e.g. to fix up what a block copy cannot get right.
static POST_INSTALL_UNIT: OnceLock<Option<String>> = OnceLock::new();

/// The disk the OS was installed to, once the installation succeeded.
static INSTALLED_NODE: Mutex<Option<String>> = Mutex::new(None);

pub fn set_post_install_unit(unit: Option<String>) {
    let _ = POST_INSTALL_UNIT.set(unit.filter(|u| !u.is_empty()));
}

pub fn installed_node() -> Option<String> {
    INSTALLED_NODE.lock().unwrap().clone()
}

/// The LUKS2 partition on `disk`, from the udev database.
pub fn luks_partition(disk: &str) -> Option<String> {
    let disk = std::fs::canonicalize(disk).ok()?;
    let name = disk.file_name()?.to_str()?.to_owned();
    let sysfs = Path::new("/sys/class/block").join(&name);

    let mut partitions: Vec<String> = std::fs::read_dir(&sysfs)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().join("partition").exists())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    partitions.sort();

    partitions.into_iter().find_map(|partition| {
        let dev = std::fs::read_to_string(sysfs.join(&partition).join("dev")).ok()?;
        let udev = std::fs::read_to_string(format!("/run/udev/data/b{}", dev.trim())).ok()?;
        udev.lines()
            .any(|line| line == "E:ID_FS_TYPE=crypto_LUKS")
            .then(|| format!("/dev/{partition}"))
    })
}

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
            &mut zlink::unix::connect(socket_path()).await?
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

    pub async fn install(&self, disk: &Disk, erase: bool) -> anyhow::Result<()> {
        let mut demo_server = if crate::is_demo() {
            Some(SysInstallDemo::run().await)
        } else {
            None
        };

        let connection = if let Some(ref mut demo_server) = demo_server {
            demo_server.connection()
        } else {
            &mut zlink::unix::connect(socket_path()).await?
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
                Ok(Ok(progress)) => {
                    let phase: InstallPhase = progress.phase.into();
                    disk.set_install_phase(phase);
                    // Only update the progress if sysinstall reported progress
                    if let Some(progress) = progress.progress {
                        disk.set_install_progress(progress);
                    }
                }
                Ok(Err(error)) => {
                    return Err(anyhow!("systemd-sysinstall failed: {error:?}"));
                }
                Err(zlink::Error::MissingParameters) => {
                    // SysInstall sends a empty reply once the installation is
                    // finished and zlink reports it as an error, so ignore it.
                }
                Err(error) => {
                    return Err(anyhow!("systemd-sysinstall failed: {error}"));
                }
            }
        }

        if let Some(unit) = POST_INSTALL_UNIT.get().cloned().flatten() {
            if crate::is_demo() {
                log::info!("Not starting {unit} since setup is run in demo mode");
            } else {
                let instance = super::systemd_manager::template_instance(&unit, &node)?;
                disk.set_install_phase(InstallPhase::InstallBootloader);
                super::systemd_manager::start_unit_and_wait(&instance).await?;
            }
        }

        *INSTALLED_NODE.lock().unwrap() = Some(node.to_string());

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
