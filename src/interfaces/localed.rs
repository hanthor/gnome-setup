/* SPDX-License-Identifier: GPL-3.0-or-later */

use zbus::{proxy, Result};

#[proxy(
    interface = "org.freedesktop.locale1",
    default_service = "org.freedesktop.locale1",
    default_path = "/org/freedesktop/locale1"
)]
pub trait Localed {
    #[zbus(property(emits_changed_signal = "true"))]
    fn locale(&self) -> Result<Vec<String>>;
    fn set_locale(&self, locale: Vec<String>, interactive: bool) -> Result<()>;
}
