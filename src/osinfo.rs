/* SPDX-License-Identifier: GPL-3.0-or-later */

use gtk::glib::{self, prelude::*};
use std::{env, sync::OnceLock};
use log;

pub fn logo() -> &'static str {
    static ONCE: OnceLock<glib::GString> = OnceLock::new();
    ONCE.get_or_init(|| {
        glib::os_info("LOGO").unwrap_or("application-x-executable".into())
    })
}

pub fn bind_logo(obj: impl IsA<glib::Object>, name: &str) {
    adw::StyleManager::default()
        .bind_property("dark", obj.upcast_ref(), name)
        .sync_create()
        .transform_to(|_, dark: bool| {
            Some(if dark {
                logo().into()
            } else {
                glib::gformat!("{}-dark", logo())
            })
        })
        .build();
}

pub fn name() -> &'static str {
    static ONCE: OnceLock<glib::GString> = OnceLock::new();
    ONCE.get_or_init(|| {
        glib::os_info("NAME").unwrap_or("Linux".into())
    })
}

pub fn is_prerelease() -> bool {
    static ONCE: OnceLock<bool> = OnceLock::new();
    *ONCE.get_or_init(|| {
        if let Ok(var) = env::var("SETUP_OVERRIDE_OS_IS_PRERELEASE") {
            match var.parse() {
                Ok(val) => return val,
                Err(e) => log::warn!("Failed to parse $SETUP_OVERRIDE_OS_IS_PRERELEASE: {e}"),
            }
        }

        glib::os_info("RELEASE_TYPE").map(|t| t == "prerelease").unwrap_or(false)
    })
}
