/* SPDX-License-Identifier: GPL-3.0-or-later */

mod systemd_crypt_enroll;

use log::{error, info};
use zlink::futures_util::StreamExt;

use systemd_crypt_enroll::{CryptEnroll as CryptEnrollTrait, EnrollMechanism};

const SOCKET_PATH: &str = "/run/systemd/io.systemd.CryptEnroll";

#[derive(Debug)]
pub struct CryptEnroll {}

impl CryptEnroll {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn make_recovery_key(&self) -> zlink::Result<String> {
        // TODO: Would be nice to have a demo implementation for the crytpenroll interface
        if crate::is_demo() {
            return Ok(
                "dehgheth-gntevib-lnlijjfc-neerulnn-nndflbbi-bdlhgvg-unvfldlv-lcrcfnbg".to_string(),
            );
        }

        let mut connection = zlink::unix::connect(SOCKET_PATH).await?;
        let result = connection.make_recovery_key().await?.unwrap();

        Ok(result.recovery_key.to_string())
    }

    pub async fn enroll_recovery_key(
        &self,
        node: &str,
        recovery_key: Option<&str>,
    ) -> zlink::Result<Option<String>> {
        // TODO: Would be nice to have a demo implementation for the crytpenroll interface
        if crate::is_demo() {
            info!("Not enrolling a recovery key since setup is run in demo mode");
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
                recovery_key,               // recovery_key
                // We enroll an empty password via SysInstall
                Some(""),                  // unlock_password
                Some(wipe_slots.as_ref()), // wipe_slots
            )
            .await?
            // FIXME: We should not unwrap here since it might fail
            .unwrap();

        Ok(result.recovery_key.map(|key| key.to_string()))
    }
}
