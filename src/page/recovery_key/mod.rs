/* SPDX-License-Identifier: GPL-3.0-or-later */

mod qr_code;

use adw::subclass::prelude::*;

use gettextrs::gettext;
use gtk::glib;
use gtk::prelude::{CheckButtonExt, WidgetExt};
use log::warn;

use crate::osinfo::OsInfoCallbacks;
use crate::page::base::{BasePage, BasePageExt, BasePageImpl};

glib::wrapper! {
    pub struct RecoveryKeyPage(ObjectSubclass<imp::RecoveryKeyPage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RecoveryKeyPage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use super::*;

    use crate::interfaces::CryptEnroll;
    use adw::prelude::AdwDialogExt;
    use glib::prelude::StaticType;
    use std::cell::OnceCell;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/recovery_key/recovery_key_page.blp")]
    pub struct RecoveryKeyPage {
        #[template_child]
        recovery_key_label: TemplateChild<gtk::Label>,
        #[template_child]
        confirm_btn: TemplateChild<gtk::CheckButton>,
        #[template_child]
        error_label: TemplateChild<gtk::Label>,
        #[template_child]
        qr_code_dialog_btn: TemplateChild<gtk::Button>,
        #[template_child]
        qr_code_dialog: TemplateChild<adw::Dialog>,
        #[template_child]
        qr_code_picture: TemplateChild<gtk::Picture>,
        recovery_key: OnceCell<String>,
        interface: OnceCell<CryptEnroll>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RecoveryKeyPage {
        const NAME: &'static str = "SetupRecoveryKeyPage";
        type Type = super::RecoveryKeyPage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            qr_code::QRCodePaintable::static_type();
            OsInfoCallbacks::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RecoveryKeyPage {
        fn constructed(&self) {
            self.parent_constructed();
            let interface = CryptEnroll::new();
            self.interface.set(interface).unwrap();

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = this)]
                self,
                async move {
                    this.set_recovery_key().await;
                }
            ));
        }
    }
    impl WidgetImpl for RecoveryKeyPage {}
    impl NavigationPageImpl for RecoveryKeyPage {}
    impl BasePageImpl for RecoveryKeyPage {}

    #[gtk::template_callbacks]
    impl RecoveryKeyPage {
        async fn set_recovery_key(&self) {
            let interface = self.interface.get().unwrap();

            let recovery_key = match interface.make_recovery_key().await {
                Ok(recovery_key) => recovery_key,
                Err(error) => {
                    warn!("Unable to create recovery key via systemd-cryptenroll: {error}");
                    self.show_error(&gettext(
                        "Could not generate a recovery key. Setup cannot continue.",
                    ));
                    return;
                }
            };

            self.recovery_key_label.set_label(&recovery_key);
            self.recovery_key.set(recovery_key).unwrap();
            self.qr_code_dialog_btn.set_sensitive(true);
            self.confirm_btn.set_sensitive(true);
        }

        fn show_error(&self, message: &str) {
            self.error_label.set_label(message);
            self.error_label.set_visible(true);
        }

        #[template_callback]
        fn show_qr_code_dialog(&self) {
            let color = self.obj().color();
            let paintable = qr_code::QRCodePaintable::new(
                // The dialog is never created when we don't have a recovery key
                self.recovery_key.get().unwrap(),
                color,
            );
            self.qr_code_picture.set_paintable(Some(&paintable));
            self.qr_code_dialog.present(Some(&*self.obj()));
        }

        #[template_callback]
        fn close_qr_code_dialog(&self) {
            self.qr_code_dialog.close();
        }

        #[template_callback]
        async fn on_done(&self) {
            let Some(recovery_key) = self.recovery_key.get() else {
                return;
            };
            // Also keeps a second click out while the first one is worked on
            if !self.confirm_btn.is_sensitive() || !self.confirm_btn.is_active() {
                return;
            }
            self.confirm_btn.set_sensitive(false);
            self.confirm_btn.set_active(false);
            self.error_label.set_visible(false);

            let node = crate::interfaces::installed_luks_node();
            log::info!("Enrolling the recovery key on {node}");
            let interface = self.interface.get().unwrap();
            match interface.enroll_recovery_key(&node, recovery_key).await {
                Ok(()) => self.obj().go_next(),
                Err(error) => {
                    warn!("Unable to enroll recovery key via systemd-cryptenroll: {error}");
                    self.show_error(&gettext(
                        "The recovery key could not be set up. Keep your backup and do not restart yet.",
                    ));
                    self.confirm_btn.set_sensitive(true);
                }
            }
        }
    }
}
