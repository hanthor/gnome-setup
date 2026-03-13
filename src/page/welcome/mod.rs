/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::subclass::prelude::*;

use gtk::glib;

use crate::osinfo;
use crate::page::base::{BasePage, BasePageExt, BasePageImpl};

glib::wrapper! {
    pub struct WelcomePage(ObjectSubclass<imp::WelcomePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl WelcomePage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/welcome/page.blp")]
    pub struct WelcomePage {
        #[template_child]
        info: TemplateChild<adw::StatusPage>,

        #[template_child]
        try_btn: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WelcomePage {
        const NAME: &'static str = "SetupWelcomePage";
        type Type = super::WelcomePage;
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

    impl ObjectImpl for WelcomePage {
        fn constructed(&self) {
            self.parent_constructed();
            osinfo::bind_logo(self.info.get(), "icon-name");
        }
    }

    impl WidgetImpl for WelcomePage {}
    impl NavigationPageImpl for WelcomePage {}
    impl BasePageImpl for WelcomePage {}

    #[gtk::template_callbacks]
    impl WelcomePage {
        #[template_callback]
        fn on_install(&self) {
            self.obj().go_next();
        }

        #[template_callback]
        fn on_try(&self) {
            todo!("Start demo session");
        }
    }
}
