/* SPDX-License-Identifier: GPL-3.0-or-later */

mod config;
mod window;
mod page;

use adw::prelude::*;
use config::{GETTEXT_PACKAGE, LOCALEDIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use adw::{glib, gio, Application};

static GRESOURCE_BYTES: &[u8] =
    gvdb_macros::include_gresource_from_dir!("/org/gnome/Setup", "data/resources");

fn main() -> glib::ExitCode {
    // Set up gettext translations
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR)
        .expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE)
        .expect("Unable to switch to the text domain");

    // Set up resources
    gio::resources_register(
        &gio::Resource::from_data(&glib::Bytes::from_static(GRESOURCE_BYTES)).unwrap(),
    );

    // Set up type needed for GtkBuilder
    page::BasePage::ensure_type();

    // Set up application
    let app = Application::builder()
        .application_id("org.gnome.Setup")
        .build();
    app.connect_activate(on_activate);
    app.run()
}

fn on_activate(app: &Application) {
    let window = if let Some(active) = app.active_window() {
        active
    } else {
        window::Window::new(app).upcast()
    };

    window.present();
}
