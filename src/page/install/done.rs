/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use gettextrs::gettext;

use crate::page::{self, BasePageImpl, BasePageExt};
use crate::osinfo;

glib::wrapper! {
    pub struct DonePage(ObjectSubclass<imp::DonePage>)
        @extends gtk::Widget, adw::NavigationPage, page::BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for DonePage {
        const NAME: &'static str = "SetupInstallDonePage";
        type Type = super::DonePage;
        type ParentType = page::BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/install/done.blp")]
    pub struct DonePage {
        #[template_child]
        info: TemplateChild<adw::StatusPage>,
        #[template_child]
        cancel_btn: TemplateChild<gtk::Button>,
    }

    impl ObjectImpl for DonePage {
        fn constructed(&self) {
            self.parent_constructed();

            osinfo::bind_logo(self.info.get(), "icon-name");
            self.cancel_btn.set_visible(crate::in_session());
            let msg = if crate::in_session() {
                gettext!(
                    "{} has been installed to this device. Reboot now to start using {} or \
                    continue to test in this evaluation environment.",
                    osinfo::name(), osinfo::name(),
                )
            } else {
                gettext!(
                    "{} has been installed to this device. Reboot now to start using {}",
                    osinfo::name(), osinfo::name(),
                )
            };
            self.info.set_description(Some(msg.as_str()));
        }
    }

    impl WidgetImpl for DonePage {}
    impl NavigationPageImpl for DonePage {}

    impl BasePageImpl for DonePage {}
}

#[gtk::template_callbacks]
impl DonePage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn on_reboot(&self) {
        todo!();
    }
}
