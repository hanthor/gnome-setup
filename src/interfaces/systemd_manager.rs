/* SPDX-License-Identifier: GPL-3.0-or-later */

use anyhow::anyhow;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};
use zbus::{proxy, Result};
use zlink::futures_util::StreamExt;

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait Manager {
    fn subscribe(&self) -> Result<()>;
    fn start_unit(&self, name: &str, mode: &str) -> Result<OwnedObjectPath>;
    #[zbus(signal)]
    fn job_removed(&self, id: u32, job: ObjectPath<'_>, unit: &str, result: &str) -> Result<()>;
}

/// Start `unit` and wait until its start job is finished.
pub async fn start_unit_and_wait(unit: &str) -> anyhow::Result<()> {
    let manager = ManagerProxy::new(crate::system_bus()).await?;
    let mut removed = manager.receive_job_removed().await?;
    // Job signals are only sent to subscribed clients
    if let Err(error) = manager.subscribe().await {
        log::debug!("Failed to subscribe to systemd signals: {error}");
    }

    let job = manager.start_unit(unit, "replace").await?;
    log::info!("Started {unit} as job {}", job.as_str());

    while let Some(signal) = removed.next().await {
        let args = signal.args()?;
        if args.job.as_str() != job.as_str() {
            continue;
        }

        return match args.result {
            "done" => Ok(()),
            result => Err(anyhow!("{unit} failed (job result: {result})")),
        };
    }

    Err(anyhow!("Lost the connection to systemd while waiting for {unit}"))
}

/// Escape a path for use as the instance of a template unit, like `systemd-escape --path`.
pub fn escape_path(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return "-".into();
    }

    let mut escaped = String::with_capacity(trimmed.len());
    for (i, byte) in trimmed.bytes().enumerate() {
        match byte {
            b'/' => escaped.push('-'),
            b'.' if i == 0 => escaped.push_str("\\x2e"),
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'.' | b':' => escaped.push(byte as char),
            _ => escaped.push_str(&format!("\\x{byte:02x}")),
        }
    }

    escaped
}

/// The instance of template `unit` (e.g. `foo@.service`) for `path`.
pub fn template_instance(unit: &str, path: &str) -> anyhow::Result<String> {
    let Some((prefix, suffix)) = unit.split_once("@.") else {
        return Err(anyhow!("{unit} is not a template unit"));
    };

    Ok(format!("{prefix}@{}.{suffix}", escape_path(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_device_nodes() {
        assert_eq!(escape_path("/dev/vdb"), "dev-vdb");
        assert_eq!(escape_path("/dev/disk/by-id/a-b"), "dev-disk-by\\x2did-a\\x2db");
        assert_eq!(
            template_instance("distro-post-install@.service", "/dev/nvme0n1").unwrap(),
            "distro-post-install@dev-nvme0n1.service"
        );
        assert!(template_instance("plain.service", "/dev/sda").is_err());
    }
}
