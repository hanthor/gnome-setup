/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::page::{self, NextTarget};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "SetupWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/window.blp")]
    pub struct Window {
        #[template_child]
        pub nav: TemplateChild<adw::NavigationView>,
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();

            let page: page::BasePage = if crate::in_session() {
                page::WelcomePage::page()
            } else {
                page::LanguagePage::page()
            };

            page.set_can_pop(false);
            self.nav.push(&page);
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

impl Window {
    pub fn new(application: &impl IsA<gtk::Application>) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}
