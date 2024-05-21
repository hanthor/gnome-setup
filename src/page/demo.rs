/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use gettextrs::gettext;

use crate::page::{self, BasePageExt};

glib::wrapper! {
    pub struct DemoPage(ObjectSubclass<imp::DemoPage>)
        @extends gtk::Widget, adw::NavigationPage, page::BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for DemoPage {
        const NAME: &'static str = "SetupDemoPage";
        type Type = super::DemoPage;
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
    #[template(file = "src/page/demo.blp")]
    pub struct DemoPage {
        #[template_child]
        info: TemplateChild<adw::StatusPage>,

        #[template_child]
        try_btn: TemplateChild<gtk::Button>,
    }

    impl ObjectImpl for DemoPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.info.set_icon_name(
                glib::os_info("ICON").as_deref().or(Some("application-x-executable"))
            );

            let os_name = glib::os_info("NAME").unwrap_or("Linux".into());

            self.info.set_title(gettext!("Welcome to {}!", os_name).as_str());
            self.try_btn.set_label(gettext!("_Try {}", os_name).as_str());
        }
    }

    impl WidgetImpl for DemoPage {}
    impl NavigationPageImpl for DemoPage {}

    impl page::BasePageImpl for DemoPage {
        fn next(&self) -> page::BasePage {
            panic!()
        }
    }
}

#[gtk::template_callbacks]
impl DemoPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn on_install(&self) {
        self.go_next(page::PrereleasePage::new().upcast_ref());
    }

    #[template_callback]
    fn on_try(&self) {
        todo!("Start demo session");
    }
}
