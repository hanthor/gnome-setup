/* SPDX-License-Identifier: GPL-3.0-or-later */

mod systemd_shutdown;

use log::{error, info};

use systemd_shutdown::Shutdown as ShutdownTrait;

const SOCKET_PATH: &str = "/run/systemd/io.systemd.Shutdown";

pub struct Shutdown {}

impl Shutdown {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn reboot(&self) -> zlink::Result<()> {
        if crate::is_demo() {
            // Do nothing
            info!("Not rebooting since setup is run in demo mode");
            return Ok(());
        }

        let mut connection = zlink::unix::connect(SOCKET_PATH).await?;

        if let Err(error) = connection
            .reboot(
                Some(false), // allow_interactive_authentication
                Some(true),  // skip_inhibitors
            )
            .await?
        {
            error!("Failed to reboot systemd: {error:?}");
        }

        Ok(())
    }
}
