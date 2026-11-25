/* SPDX-License-Identifier: GPL-3.0-or-later */

mod systemd_crypt_enroll;

use std::cell::Cell;
use std::sync::{Mutex, OnceLock};

use anyhow::{anyhow, bail, ensure};
use log::info;
use zlink::futures_util::StreamExt;

use systemd_crypt_enroll::{CryptEnroll as CryptEnrollTrait, EnrollMechanism};

const DEFAULT_SOCKET_PATH: &str = "/run/systemd/io.systemd.CryptEnroll";

static SOCKET_PATH: OnceLock<String> = OnceLock::new();

pub fn set_crypt_enroll_socket(path: Option<String>) {
    if let Some(path) = path.filter(|path| !path.is_empty()) {
        let _ = SOCKET_PATH.set(path);
    }
}

fn socket_path() -> &'static str {
    SOCKET_PATH.get().map_or(DEFAULT_SOCKET_PATH, String::as_str)
}

/// The key slot of the passphrase the user chose, which must survive wiping the empty password
/// SysInstall enrolled.
static PASSPHRASE_SLOT: Mutex<Option<i64>> = Mutex::new(None);

/// Whether the machine has a TPM 2.0 the installed disk can be bound to. Without one the disk
/// needs a passphrase to unlock at boot.
pub fn has_tpm2() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/tpm") else {
        return false;
    };
    entries.flatten().any(|entry| {
        std::fs::read_to_string(entry.path().join("tpm_version_major"))
            .is_ok_and(|version| version.trim() == "2")
    })
}

/// The LUKS2 partition on the disk we just installed to, falling back to the root partition
/// label of the GNOME OS definitions.
pub fn installed_luks_node() -> String {
    crate::interfaces::installed_node()
        .and_then(|disk| crate::interfaces::luks_partition(&disk))
        .unwrap_or_else(|| "/dev/disk/by-label/root".to_string())
}

/// How far enrolling the recovery key got, so that a retry never enrolls a second key.
#[derive(Clone, Copy, Debug, Default)]
enum Enrollment {
    #[default]
    NotStarted,
    /// The request was sent, but no valid reply arrived
    Unconfirmed,
    Confirmed {
        recovery_slot: i64,
    },
}

#[derive(Debug, Default)]
pub struct CryptEnroll {
    enrollment: Cell<Enrollment>,
}

impl CryptEnroll {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn make_recovery_key(&self) -> anyhow::Result<String> {
        // TODO: Would be nice to have a demo implementation for the crytpenroll interface
        if crate::is_demo() {
            return Ok(
                "dehgheth-gntevib-lnlijjfc-neerulnn-nndflbbi-bdlhgvg-unvfldlv-lcrcfnbg".to_string(),
            );
        }

        let mut connection = zlink::unix::connect(socket_path()).await?;
        let result = connection
            .make_recovery_key()
            .await?
            .map_err(|error| anyhow::anyhow!("MakeRecoveryKey failed: {error:?}"))?;

        Ok(result.recovery_key.to_string())
    }

    /// Enroll the passphrase the user chose next to the empty password SysInstall enrolled,
    /// which [`Self::enroll_recovery_key`] wipes later on.
    pub async fn enroll_passphrase(&self, node: &str, passphrase: &str) -> anyhow::Result<()> {
        if crate::is_demo() {
            info!("Not enrolling a passphrase since setup is run in demo mode");
            return Ok(());
        }

        let mut connection = zlink::unix::connect(socket_path()).await?;
        let result = connection
            .enroll(
                node,                       // node
                &EnrollMechanism::Password, // mechanism
                None,                       // recovery_key
                // We enroll an empty password via SysInstall
                Some(""),         // unlock_password
                Some(passphrase), // password
                None,             // wipe_slots
            )
            .await?
            .map_err(|error| anyhow::anyhow!("Enroll on {node} failed: {error:?}"))?;

        *PASSPHRASE_SLOT.lock().unwrap() = result.slot;
        Ok(())
    }

    /// Enroll the recovery key, wipe the empty password SysInstall enrolled, and verify both
    /// against the key slots on the disk. Only a disk that can be unlocked with the recovery key
    /// and no longer with the empty password is a success. Can be called again after an error.
    pub async fn enroll_recovery_key(&self, node: &str, recovery_key: &str) -> anyhow::Result<()> {
        // TODO: Would be nice to have a demo implementation for the crytpenroll interface
        if crate::is_demo() {
            info!("Not enrolling a recovery key since setup is run in demo mode");
            return Ok(());
        }

        let mut connection = zlink::unix::connect(socket_path()).await?;
        let passphrase_slot = *PASSPHRASE_SLOT.lock().unwrap();

        let recovery_slot = match self.enrollment.get() {
            Enrollment::Unconfirmed => {
                bail!("A previous recovery key enrollment could not be confirmed")
            }
            Enrollment::Confirmed { recovery_slot } => recovery_slot,
            Enrollment::NotStarted => {
                // TODO: Use empty slot once https://github.com/systemd/systemd/pull/42645 is merged
                let wipe_slots =
                    temporary_slots(&list_slots(&mut connection, node).await?, passphrase_slot);
                ensure!(!wipe_slots.is_empty(), "No temporary password slot on {node}");

                // A lost reply may mean that the key was enrolled
                self.enrollment.set(Enrollment::Unconfirmed);
                let reply = connection
                    .enroll(
                        node,                       // node
                        &EnrollMechanism::Recovery, // mechanism
                        Some(recovery_key),         // recovery_key
                        // We enroll an empty password via SysInstall
                        Some(""),                  // unlock_password
                        None,                      // password
                        Some(wipe_slots.as_ref()), // wipe_slots
                    )
                    .await?
                    .map_err(|error| anyhow!("Enroll on {node} failed: {error:?}"))?;

                ensure!(
                    reply.recovery_key == Some(recovery_key),
                    "CryptEnroll returned a different recovery key"
                );
                let recovery_slot = reply
                    .slot
                    .ok_or_else(|| anyhow!("CryptEnroll did not report the recovery key slot"))?;
                self.enrollment.set(Enrollment::Confirmed { recovery_slot });
                recovery_slot
            }
        };

        verify_slots(
            &list_slots(&mut connection, node).await?,
            recovery_slot,
            passphrase_slot,
        )
    }
}

type Slot = (i64, Option<EnrollMechanism>);

async fn list_slots(
    connection: &mut zlink::unix::Connection,
    node: &str,
) -> anyhow::Result<Vec<Slot>> {
    let mut slots = Vec::new();
    let mut stream = connection.list_slots(node).await?;
    while let Some(result) = stream.next().await {
        match result {
            Ok(Ok(reply)) => {
                let slot =
                    reply.slot.ok_or_else(|| anyhow!("ListSlots returned an invalid slot"))?;
                slots.push((slot, reply.r#type));
            }
            // The server ends the list with a successful reply without parameters
            Err(zlink::Error::MissingParameters) => break,
            Ok(Err(error)) => bail!("ListSlots on {node} failed: {error:?}"),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(slots)
}

/// The password slots that are not the passphrase the user chose.
fn temporary_slots(slots: &[Slot], passphrase_slot: Option<i64>) -> Vec<i64> {
    slots
        .iter()
        .filter(|(slot, kind)| {
            *kind == Some(EnrollMechanism::Password) && Some(*slot) != passphrase_slot
        })
        .map(|(slot, _)| *slot)
        .collect()
}

fn verify_slots(
    slots: &[Slot],
    recovery_slot: i64,
    passphrase_slot: Option<i64>,
) -> anyhow::Result<()> {
    ensure!(
        slots.contains(&(recovery_slot, Some(EnrollMechanism::Recovery))),
        "The recovery key is not on the disk"
    );
    ensure!(
        temporary_slots(slots, passphrase_slot).is_empty(),
        "The temporary password is still on the disk"
    );
    if let Some(slot) = passphrase_slot {
        ensure!(
            slots.contains(&(slot, Some(EnrollMechanism::Password))),
            "The disk passphrase is not on the disk"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: Option<EnrollMechanism> = Some(EnrollMechanism::Password);
    const RECOVERY: Option<EnrollMechanism> = Some(EnrollMechanism::Recovery);
    const TPM2: Option<EnrollMechanism> = Some(EnrollMechanism::Tpm2);

    #[test]
    fn only_the_temporary_password_is_wiped() {
        let slots = [(0, PASSWORD), (1, TPM2), (2, PASSWORD)];
        assert_eq!(temporary_slots(&slots, Some(2)), [0]);
        assert_eq!(temporary_slots(&slots, None), [0, 2]);
    }

    #[test]
    fn a_disk_needs_the_recovery_key_and_no_temporary_password() {
        assert!(verify_slots(&[(1, TPM2), (2, RECOVERY)], 2, None).is_ok());
        assert!(verify_slots(&[(1, PASSWORD), (2, RECOVERY)], 2, Some(1)).is_ok());
        // Recovery key missing, or in another slot
        assert!(verify_slots(&[(1, TPM2)], 2, None).is_err());
        assert!(verify_slots(&[(3, RECOVERY)], 2, None).is_err());
        // Temporary password left
        assert!(verify_slots(&[(0, PASSWORD), (2, RECOVERY)], 2, None).is_err());
        assert!(verify_slots(&[(0, PASSWORD), (1, PASSWORD), (2, RECOVERY)], 2, Some(1)).is_err());
        // The user's passphrase is gone
        assert!(verify_slots(&[(2, RECOVERY)], 2, Some(1)).is_err());
    }
}
