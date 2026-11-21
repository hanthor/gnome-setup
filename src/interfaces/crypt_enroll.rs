/* SPDX-License-Identifier: GPL-3.0-or-later */

mod systemd_crypt_enroll;

use std::sync::Mutex;

use log::{error, info};
use zlink::futures_util::StreamExt;

use systemd_crypt_enroll::{CryptEnroll as CryptEnrollTrait, EnrollMechanism};

const SOCKET_PATH: &str = "/run/systemd/io.systemd.CryptEnroll";

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

#[derive(Debug)]
pub struct CryptEnroll {}

impl CryptEnroll {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn make_recovery_key(&self) -> anyhow::Result<String> {
        // TODO: Would be nice to have a demo implementation for the crytpenroll interface
        if crate::is_demo() {
            return Ok(
                "dehgheth-gntevib-lnlijjfc-neerulnn-nndflbbi-bdlhgvg-unvfldlv-lcrcfnbg".to_string(),
            );
        }

        let mut connection = zlink::unix::connect(SOCKET_PATH).await?;
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

        let mut connection = zlink::unix::connect(SOCKET_PATH).await?;
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

    pub async fn enroll_recovery_key(
        &self,
        node: &str,
        recovery_key: Option<&str>,
    ) -> anyhow::Result<Option<String>> {
        // TODO: Would be nice to have a demo implementation for the crytpenroll interface
        if crate::is_demo() {
            info!("Not enrolling a recovery key since setup is run in demo mode");
            return Ok(Some(
                "dehgheth-gntevib-lnlijjfc-neerulnn-nndflbbi-bdlhgvg-unvfldlv-lcrcfnbg".to_string(),
            ));
        }

        let mut connection = zlink::unix::connect(SOCKET_PATH).await?;

        let passphrase_slot = *PASSPHRASE_SLOT.lock().unwrap();
        let mut stream = connection.list_slots(node).await?;
        let mut wipe_slots = Vec::with_capacity(1);
        while let Some(result) = stream.next().await {
            let reply = match result {
                Ok(Ok(reply)) => reply,
                Ok(Err(error)) => {
                    error!("Unexpected response while fetching key slots:  {error:?}");
                    continue;
                }
                Err(error) => {
                    error!("Connection to CryptEnroll failed: {error:?}");
                    continue;
                }
            };
            // TODO: Use empty slot once https://github.com/systemd/systemd/pull/42645 is merged
            if reply.r#type == Some(EnrollMechanism::Password) {
                if let Some(slot) = reply.slot.filter(|slot| Some(*slot) != passphrase_slot) {
                    wipe_slots.push(slot)
                }
            }
        }

        drop(stream);

        let result = connection
            .enroll(
                node,                       // node
                &EnrollMechanism::Recovery, // mechanism
                recovery_key,               // recovery_key
                // We enroll an empty password via SysInstall
                Some(""),                  // unlock_password
                None,                      // password
                Some(wipe_slots.as_ref()), // wipe_slots
            )
            .await?
            .map_err(|error| anyhow::anyhow!("Enroll on {node} failed: {error:?}"))?;

        Ok(result.recovery_key.map(|key| key.to_string()))
    }
}
