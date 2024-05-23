/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use gettextrs::gettext;

use crate::page::{self, BasePageImpl, BasePageExt};
use crate::osinfo;

glib::wrapper! {
    pub struct PrereleasePage(ObjectSubclass<imp::PrereleasePage>)
        @extends gtk::Widget, adw::NavigationPage, page::BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for PrereleasePage {
        const NAME: &'static str = "SetupPrereleasePage";
        type Type = super::PrereleasePage;
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
    #[template(file = "src/page/prerelease/prerelease.blp")]
    pub struct PrereleasePage {
        #[template_child]
        status: TemplateChild<adw::StatusPage>,
    }

    impl ObjectImpl for PrereleasePage {
        fn constructed(&self) {
            self.parent_constructed();

            // TRANSLATORS: The first {} is the name of the OS
            self.status.set_description(Some(gettext!(
                "This is an early development build of {}. Please be aware that breaking \
                changes may still occur. You may need to manually migrate data after such a \
                change. We provide no stability or security guarantees. Back up all important \
                data. Do not use in a production environment.",
                osinfo::name(),
            ).as_str()));
        }
    }

    impl WidgetImpl for PrereleasePage {}
    impl NavigationPageImpl for PrereleasePage {}

    impl BasePageImpl for PrereleasePage {}
}

type Next = page::EulaPage;

#[gtk::template_callbacks]
impl PrereleasePage {
    pub fn new() -> page::BasePage {
        if osinfo::is_prerelease() {
            glib::Object::new::<Self>().upcast()
        } else {
            Next::new().upcast()
        }
    }

    #[template_callback]
    fn next(&self) {
        self.go_next(Next::new());
    }
}
