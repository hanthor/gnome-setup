/* SPDX-License-Identifier: GPL-3.0-or-later */

mod systemd_sysinstall;
mod systemd_sysinstall_demo;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Definitions the distribution offers for installing without encryption, next to the
/// compiled-in (encrypted) ones. Only when they are set can the user turn encryption off.
static UNENCRYPTED_DEFINITIONS: OnceLock<Option<String>> = OnceLock::new();

/// Whether the user wants the installed disk encrypted.
static ENCRYPT_DISK: AtomicBool = AtomicBool::new(true);

pub fn set_unencrypted_definitions(dir: Option<String>) {
    let _ = UNENCRYPTED_DEFINITIONS.set(dir.filter(|d| !d.is_empty()));
}

fn unencrypted_definitions() -> Option<&'static str> {
    UNENCRYPTED_DEFINITIONS.get().and_then(Option::as_deref)
}

/// Whether the user can choose to install without encryption.
pub fn encryption_optional() -> bool {
    unencrypted_definitions().is_some()
}

pub fn set_encrypt_disk(encrypt: bool) {
    ENCRYPT_DISK.store(encrypt, Ordering::Relaxed);
}

/// Whether the installed disk will be encrypted.
pub fn encrypt_disk() -> bool {
    will_encrypt(ENCRYPT_DISK.load(Ordering::Relaxed), unencrypted_definitions())
}

/// Without unencrypted definitions the disk is always encrypted, whatever was chosen.
fn will_encrypt(encrypt: bool, unencrypted: Option<&str>) -> bool {
    encrypt || unencrypted.is_none()
}

/// The definitions systemd-sysinstall installs with.
fn definitions_dir(encrypt: bool, unencrypted: Option<&str>) -> &str {
    match unencrypted {
        Some(dir) if !will_encrypt(encrypt, unencrypted) => dir,
        _ => REPART_DEFINITIONS_DIR,
    }
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
            partition_count,
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
            // currentSizeBytes is the whole device, so only the partition count tells an empty
            // disk apart. Without it (an older systemd-sysinstall), don't call a disk empty.
            DeviceFit::EnoughFreeSpace => match partition_count {
                Some(0) => DiskStatus::Empty,
                _ => DiskStatus::DataDetected,
            },
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
        let encrypt = ENCRYPT_DISK.load(Ordering::Relaxed);
        let definitions = [definitions_dir(encrypt, unencrypted_definitions())];
        log::info!("Installing to {node} with the definitions in {}", definitions[0]);

        let mut stream = connection
            .run_with_progress(
                &node,                        // node
                &definitions,                 // definitions
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

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: i64 = 512 * 1000 * 1000 * 1000;

    fn status(partition_count: Option<i64>, current_size_bytes: Option<i64>) -> DiskStatus {
        Disk::from_candidate_device(CandidateDevice {
            node: Some("/dev/vda".to_string()),
            size_bytes: Some(SIZE),
            fit: Some(DeviceFit::EnoughFreeSpace),
            current_size_bytes,
            partition_count,
            ..CandidateDevice::default()
        })
        .unwrap()
        .status()
    }

    #[test]
    fn a_disk_without_partitions_is_empty() {
        // systemd-repart reports the whole device as its current size
        assert_eq!(status(Some(0), Some(SIZE)), DiskStatus::Empty);
        assert_eq!(status(Some(0), None), DiskStatus::Empty);
    }

    #[test]
    fn a_disk_with_partitions_has_data() {
        assert_eq!(status(Some(3), Some(SIZE)), DiskStatus::DataDetected);
        assert_eq!(status(Some(3), Some(0)), DiskStatus::DataDetected);
    }

    #[test]
    fn a_disk_is_not_empty_without_a_partition_count() {
        // An older systemd-sysinstall that doesn't send partitionCount
        assert_eq!(status(None, Some(SIZE)), DiskStatus::DataDetected);
        assert_eq!(status(None, Some(0)), DiskStatus::DataDetected);
    }

    #[test]
    fn a_whole_disk_filesystem_without_partition_table_has_data() {
        // systemd-sysinstall leaves partitionCount out for a disk without a partition table
        // that still carries a signature (filesystem, LUKS, LVM, ...)
        assert_eq!(status(None, Some(SIZE)), DiskStatus::DataDetected);
    }

    #[test]
    fn the_disk_is_encrypted_unless_the_user_turns_it_off() {
        assert!(will_encrypt(true, None));
        assert!(will_encrypt(true, Some("/run/unencrypted.d")));
        assert!(!will_encrypt(false, Some("/run/unencrypted.d")));
        // Without unencrypted definitions there is nothing to turn off
        assert!(will_encrypt(false, None));
    }

    #[test]
    fn only_an_unencrypted_install_uses_the_unencrypted_definitions() {
        assert_eq!(definitions_dir(true, None), REPART_DEFINITIONS_DIR);
        assert_eq!(definitions_dir(false, None), REPART_DEFINITIONS_DIR);
        assert_eq!(definitions_dir(true, Some("/run/unencrypted.d")), REPART_DEFINITIONS_DIR);
        assert_eq!(definitions_dir(false, Some("/run/unencrypted.d")), "/run/unencrypted.d");
    }
}
