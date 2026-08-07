/* SPDX-License-Identifier: GPL-3.0-or-later */

mod systemd_crypt_enroll;

use log::error;
use zlink::futures_util::StreamExt;

use systemd_crypt_enroll::{CryptEnroll as CryptEnrollTrait, EnrollMechanism};

const SOCKET_PATH: &str = "/run/systemd/io.systemd.CryptEnroll";

pub struct CryptEnroll {}

impl CryptEnroll {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn enroll_recovery_key(&self, node: &str) -> zlink::Result<Option<String>> {
        // TODO: Would be nice to have a demo implementation for the crytpenroll interface
        if crate::is_demo() {
            return Ok(Some(
                "dehgheth-gntevib-lnlijjfc-neerulnn-nndflbbi-bdlhgvg-unvfldlv-lcrcfnbg".to_string(),
            ));
        }

        let mut connection = zlink::unix::connect(SOCKET_PATH).await?;

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
                if let Some(slot) = reply.slot {
                    wipe_slots.push(slot)
                }
            }
        }

        drop(stream);

        let result = connection
            .enroll(
                node,                       // node
                &EnrollMechanism::Recovery, // mechanism
                // We enroll an empty password via SysInstall
                Some(""),                  // unlock_password
                Some(wipe_slots.as_ref()), // wipe_slots
            )
            .await?
            .unwrap();

        Ok(result.recovery_key.map(|key| key.to_string()))
    }
}
