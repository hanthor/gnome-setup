/* SPDX-License-Identifier: GPL-3.0-or-later */

use formatx::formatx;
use glib::prelude::*;

use std::env;
use std::sync::OnceLock;

pub fn logo() -> &'static str {
    static ONCE: OnceLock<glib::GString> = OnceLock::new();
    ONCE.get_or_init(|| glib::os_info("LOGO").unwrap_or("org.gnome.Setup".into()))
}

pub fn bind_logo(obj: impl IsA<glib::Object>, name: &str) {
    adw::StyleManager::default()
        .bind_property("dark", obj.upcast_ref(), name)
        .sync_create()
        .transform_to(|_, dark: bool| {
            let mut maybe_dark: Option<glib::GString> = None;
            if dark {
                let theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap());
                let name = glib::gformat!("{}-dark", logo());
                if theme.has_icon(&name) {
                    maybe_dark = Some(name);
                }
            }
            maybe_dark.or_else(|| Some(logo().into()))
        })
        .build();
}

pub fn name() -> &'static str {
    static ONCE: OnceLock<glib::GString> = OnceLock::new();
    ONCE.get_or_init(|| glib::os_info("NAME").unwrap_or("Linux".into()))
}

pub enum ReleaseType {
    Experimental { description: Option<String> },
    PreRelease,
    Stable,
    Lts,
}

pub fn release_type() -> &'static ReleaseType {
    static ONCE: OnceLock<ReleaseType> = OnceLock::new();
    ONCE.get_or_init(|| {
        let reltype = env::var("SETUP_OVERRIDE_OS_RELEASE_TYPE")
            .ok()
            .map(|s| s.into())
            .or_else(|| glib::os_info("RELEASE_TYPE"));

        match reltype.as_deref() {
            Some("stable") | None => ReleaseType::Stable,
            Some("lts") => ReleaseType::Lts,
            Some("pre-release") => ReleaseType::PreRelease,
            Some("experiment") => ReleaseType::Experimental {
                description: env::var("SETUP_OVERRIDE_OS_EXPERIMENT")
                    .ok()
                    .or_else(|| glib::os_info("EXPERIMENT").map(Into::into)),
            },
            Some(other) => {
                log::warn!("Encountered unknown RELEASE_TYPE: {other}");
                ReleaseType::Stable
            }
        }
    })
}

pub struct OsInfoCallbacks {}

#[gtk::template_callbacks(functions)]
impl OsInfoCallbacks {
    #[template_callback]
    fn apply_osinfo(string: &str) -> String {
        formatx!(string, os_name = name()).unwrap()
    }

    #[template_callback]
    fn in_session(value1: glib::Value, value2: glib::Value) -> glib::Value {
        let result = if crate::in_session() { value1 } else { value2 };

        if let Ok(string) = result.get::<&str>() {
            Self::apply_osinfo(string).into()
        } else {
            result
        }
    }

    #[template_callback]
    fn release_type(
        experimental: glib::Value,
        pre_rlease: glib::Value,
        stable: glib::Value,
        lts: glib::Value,
    ) -> glib::Value {
        let result = match release_type() {
            ReleaseType::Experimental { .. } => experimental,
            ReleaseType::PreRelease => pre_rlease,
            ReleaseType::Stable => stable,
            ReleaseType::Lts => lts,
        };

        if let Ok(string) = result.get::<&str>() {
            Self::apply_osinfo(string).into()
        } else {
            result
        }
    }
}
