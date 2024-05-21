/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use crate::page;

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
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/prerelease.blp")]
    pub struct PrereleasePage {
    }

    impl ObjectImpl for PrereleasePage {}
    impl WidgetImpl for PrereleasePage {}
    impl NavigationPageImpl for PrereleasePage {}

    impl page::BasePageImpl for PrereleasePage {
        fn next(&self) -> page::BasePage {
            page::EulaPage::new().upcast()
        }
    }
}

impl PrereleasePage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
