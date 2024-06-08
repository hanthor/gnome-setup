/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use gettextrs::gettext;
use gtk::glib;

use crate::osinfo;

glib::wrapper! {
    pub struct DonePage(ObjectSubclass<imp::DonePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for DonePage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl DonePage {
    pub fn page() -> BasePage {
        Self::default().upcast()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/install/done_page.blp")]
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
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DonePage {
        fn constructed(&self) {
            self.parent_constructed();

            self.cancel_btn.set_visible(crate::in_session());
            let msg = if crate::in_session() {
                gettext!(
                    "{} has been installed to this device. Reboot now to start using {} or \
                    continue to test in this evaluation environment.",
                    osinfo::name(),
                    osinfo::name(),
                )
            } else {
                gettext!(
                    "{} has been installed to this device. Reboot now to start using {}",
                    osinfo::name(),
                    osinfo::name(),
                )
            };
            self.info.set_description(Some(msg.as_str()));
        }
    }

    impl WidgetImpl for DonePage {}
    impl NavigationPageImpl for DonePage {}
    impl BasePageImpl for DonePage {}

    #[gtk::template_callbacks]
    impl DonePage {
        #[template_callback]
        fn on_reboot(&self) {
            todo!();
        }
    }
}
