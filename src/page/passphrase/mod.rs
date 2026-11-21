/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use log::warn;

use crate::osinfo;
use crate::page::base::{BasePage, BasePageExt, BasePageImpl, BasePagePropertiesExt};

glib::wrapper! {
    pub struct PassphrasePage(ObjectSubclass<imp::PassphrasePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PassphrasePage {
    /// Only machines without a TPM 2.0 need a passphrase: with one, the disk unlocks itself.
    pub fn new() -> Option<Self> {
        if crate::interfaces::has_tpm2() {
            None
        } else {
            Some(glib::Object::new())
        }
    }
}

mod imp {
    use super::*;

    use crate::interfaces::CryptEnroll;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/passphrase/page.blp")]
    pub struct PassphrasePage {
        #[template_child]
        passphrase_row: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        confirm_row: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        error_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PassphrasePage {
        const NAME: &'static str = "SetupPassphrasePage";
        type Type = super::PassphrasePage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            osinfo::OsInfoCallbacks::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PassphrasePage {}
    impl WidgetImpl for PassphrasePage {}
    impl NavigationPageImpl for PassphrasePage {}
    impl BasePageImpl for PassphrasePage {}

    #[gtk::template_callbacks]
    impl PassphrasePage {
        fn show_error(&self, message: &str) {
            self.error_label.set_label(message);
            self.error_label.set_visible(true);
        }

        #[template_callback]
        fn update_ready(&self) {
            let passphrase = self.passphrase_row.text();
            let confirm = self.confirm_row.text();
            let matches = passphrase == confirm;

            self.error_label.set_visible(false);
            if !matches && !confirm.is_empty() {
                self.show_error(&gettext("The passphrases do not match."));
            }
            self.obj().set_ready(!passphrase.is_empty() && matches);
        }

        #[template_callback]
        async fn next(&self) {
            let obj = self.obj();
            if !obj.ready() {
                return;
            }
            obj.set_ready(false);

            let node = crate::interfaces::installed_luks_node();
            log::info!("Enrolling the disk passphrase on {node}");
            let passphrase = self.passphrase_row.text();
            match CryptEnroll::new().enroll_passphrase(&node, &passphrase).await {
                Ok(()) => obj.go_next(),
                Err(error) => {
                    warn!("Unable to enroll the passphrase via systemd-cryptenroll: {error}");
                    self.show_error(&gettext("The passphrase could not be set."));
                    obj.set_ready(true);
                }
            }
        }
    }
}
