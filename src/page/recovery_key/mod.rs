/* SPDX-License-Identifier: GPL-3.0-or-later */

mod qr_code;

use adw::subclass::prelude::*;

use gtk::glib;
use gtk::prelude::WidgetExt;
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
        done_btn: TemplateChild<gtk::Button>,
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
                    return;
                }
            };

            self.recovery_key_label.set_label(&recovery_key);
            self.recovery_key.set(recovery_key).unwrap();
            self.qr_code_dialog_btn.set_sensitive(true);
        }

        async fn enroll_recovery_key(&self) {
            // FIXME: Get the node we need to set the recovery key dynamically
            let node = "/dev/disk/by-label/root";
            let recovery_key = self.recovery_key.get();
            let interface = self.interface.get().unwrap();

            let result =
                interface.enroll_recovery_key(node, recovery_key.map(|x| x.as_str())).await;
            match result {
                Ok(Some(_)) => {}
                Ok(None) => warn!("Systemd-cryptenroll didn't report a recovery key"),
                Err(error) => {
                    warn!("Unable to enroll recovery key via systemd-cryptenroll: {error}")
                }
            }
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
            self.enroll_recovery_key().await;

            self.obj().go_next();
        }
    }
}
