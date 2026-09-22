/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::subclass::prelude::*;

use gtk::glib;

use crate::osinfo::OsInfoCallbacks;
use crate::page::base::{BasePage, BasePageImpl};

glib::wrapper! {
    pub struct DonePage(ObjectSubclass<imp::DonePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DonePage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use super::*;

    use log::error;

    use crate::interfaces::Shutdown;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/done/done_page.blp")]
    pub struct DonePage {
        #[template_child]
        info: TemplateChild<adw::StatusPage>,
        #[template_child]
        cancel_btn: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DonePage {
        const NAME: &'static str = "SetupInstallDonePage";
        type Type = super::DonePage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            OsInfoCallbacks::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DonePage {}
    impl WidgetImpl for DonePage {}
    impl NavigationPageImpl for DonePage {}
    impl BasePageImpl for DonePage {}

    #[gtk::template_callbacks]
    impl DonePage {
        #[template_callback]
        async fn on_reboot(&self) {
            let interface = Shutdown::new();

            if let Err(error) = interface.reboot().await {
                error!("Failed to reboot system: {error}");
            }
        }
    }
}
