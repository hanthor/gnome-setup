/* SPDX-License-Identifier: GPL-3.0-or-later */

mod config;
mod interfaces;
mod models;
mod osinfo;
mod page;
mod window;

use adw::prelude::*;

use std::env;
use std::sync::OnceLock;

use adw::{gio, Application};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};

use config::{GETTEXT_PACKAGE, LOCALEDIR};

fn main() -> glib::ExitCode {
    // Set up gettext translations
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    // Set up logging
    static LOGGER: glib::GlibLogger = glib::GlibLogger::new(
        glib::GlibLoggerFormat::Plain,
        glib::GlibLoggerDomain::CrateTarget,
    );
    log::set_logger(&LOGGER).expect("Failed to set up logging");
    log::set_max_level(log::LevelFilter::Debug);

    // Set up resources
    static GRESOURCE_BYTES: &[u8] =
        gvdb_macros::include_gresource_from_dir!("/org/gnome/Setup", "data/resources");
    gio::resources_register(
        &gio::Resource::from_data(&glib::Bytes::from_static(GRESOURCE_BYTES)).unwrap(),
    );

    // Set up type needed for GtkBuilder
    page::BasePage::ensure_type();

    // Set up application
    let app = Application::builder().application_id("org.gnome.Setup").build();
    app.connect_activate(on_activate);
    app.run()
}

fn on_activate(app: &Application) {
    // Force the app into light mode, for testing purposes
    // TODO: DROP THIS EVENTUALLY. User can technically switch gnome-shell into dark mode from
    // quick settings, and maybe one day we'll want a light/dark mode switch in initial setup
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);

    let window = if let Some(active) = app.active_window() {
        active
    } else {
        window::Window::new(app).upcast()
    };

    window.present();
}

pub fn in_session() -> bool {
    static ONCE: OnceLock<bool> = OnceLock::new();
    *ONCE.get_or_init(|| {
        if let Ok(var) = env::var("SETUP_OVERRIDE_IN_SESSION") {
            match var.parse() {
                Ok(val) => return val,
                Err(e) => log::warn!("Failed to parse $SETUP_OVERRIDE_IN_SESSION: {e}"),
            }
        }

        env::var("XDG_SESSION_CLASS").map(|val| val == "user").unwrap_or(false)
    })
}

pub fn is_demo() -> bool {
    static ONCE: OnceLock<bool> = OnceLock::new();
    *ONCE.get_or_init(|| {
        let Ok(var) = env::var("SETUP_DEMO") else {
            return false;
        };
        match var.parse() {
            Ok(val) => val,
            Err(e) => {
                log::warn!("Failed to parse $SETUP_OVERRIDE_IN_SESSION: {e}");
                false
            }
        }
    })
}

pub fn system_bus() -> &'static zbus::Connection {
    static ONCE: OnceLock<zbus::Connection> = OnceLock::new();
    ONCE.get_or_init(|| zbus::blocking::Connection::system().unwrap().into_inner())
}

pub fn locale() -> String {
    // https://github.com/Koka/gettext-rs/issues/96
    let cstr = unsafe {
        let out = gettext_sys::setlocale(
            gettextrs::LocaleCategory::LcMessages as i32,
            std::ptr::null(),
        );
        assert!(!out.is_null());
        std::ffi::CStr::from_ptr(out)
    };
    String::from_utf8_lossy(cstr.to_bytes()).to_string()
}
