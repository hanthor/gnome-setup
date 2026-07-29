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
    fn set_x11_keyboard(
        &self,
        layout: &str,
        model: &str,
        variant: &str,
        options: &str,
        convert: bool,
        interactive: bool,
    ) -> Result<()>;
    #[zbus(property)]
    fn x11_layout(&self) -> Result<String>;
    #[zbus(property)]
    fn x11_variant(&self) -> Result<String>;
}

impl LocaledProxy<'_> {
    pub async fn current_keyboards(&self) -> Vec<String> {
        let layouts = match self.x11_layout().await {
            Ok(layouts) => layouts,
            Err(error) => {
                log::error!("Failed to get keyboard layout from localed: {error}");
                return Vec::new();
            }
        };

        let variants = match self.x11_variant().await {
            Ok(variants) => variants,
            Err(error) => {
                log::error!("Failed to get keyboard variant from localed: {error}");
                String::new()
            }
        };

        let layouts = layouts.split(",");
        let variants = variants.split(",");

        layouts
            .zip(variants.chain(std::iter::repeat("")))
            .map(|(layout, variant)| {
                if variant.is_empty() {
                    layout.to_string()
                } else {
                    format!("{layout}+{variant}")
                }
            })
            .collect()
    }
}
