/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib};

use std::{cell::Cell, marker::PhantomData};
use crate::page::{SetupPage, SetupPageImpl};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/prerelease.blp")]
    pub struct SetupPrereleasePage {
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupPrereleasePage {
        const NAME: &'static str = "SetupPrereleasePage";
        type Type = super::SetupPrereleasePage;
        type ParentType = SetupPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupPrereleasePage {}
    impl WidgetImpl for SetupPrereleasePage {}
    impl NavigationPageImpl for SetupPrereleasePage {}

    impl SetupPageImpl for SetupPrereleasePage {
        fn next(&self) -> SetupPage {
            crate::page::SetupEulaPage::new().upcast()
        }
    }
}

glib::wrapper! {
    pub struct SetupPrereleasePage(ObjectSubclass<imp::SetupPrereleasePage>)
        @extends gtk::Widget, adw::NavigationPage, SetupPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupPrereleasePage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
