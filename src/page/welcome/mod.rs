/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use crate::page::subclass::*;

use gtk::glib;
use gettextrs::gettext;

use crate::osinfo;
use crate::page::PrereleasePage as Next;

glib::wrapper! {
    pub struct WelcomePage(ObjectSubclass<imp::WelcomePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for WelcomePage {
        const NAME: &'static str = "SetupWelcomePage";
        type Type = super::WelcomePage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/welcome/page.blp")]
    pub struct WelcomePage {
        #[template_child]
        info: TemplateChild<adw::StatusPage>,

        #[template_child]
        try_btn: TemplateChild<gtk::Button>,
    }

    impl ObjectImpl for WelcomePage {
        fn constructed(&self) {
            self.parent_constructed();

            osinfo::bind_logo(self.info.get(), "icon-name");

            if crate::in_session() {
                self.obj().set_title(gettext("Welcome").as_str());
                self.info.set_title(gettext("Ready to Install?").as_str());
                self.info.set_description(Some(gettext!(
                    "You can start an installation while you continue testing {}",
                    osinfo::name(),
                ).as_str()));
                self.try_btn.set_visible(false);
            } else {
                self.obj().set_title(gettext("Try or Install?").as_str());
                self.info.set_title(gettext!("Welcome to {}!", osinfo::name()).as_str());
                self.info.set_description(Some(gettext!(
                    "You can try {} in a temporary environment before you install, or \
                    you can start an installation right away.",
                    osinfo::name(),
                ).as_str()));
                self.try_btn.set_label(gettext!("_Try {}", osinfo::name()).as_str());
            }
        }
    }

    impl WidgetImpl for WelcomePage {}
    impl NavigationPageImpl for WelcomePage {}

    impl BasePageImpl for WelcomePage {}
}

#[gtk::template_callbacks]
impl WelcomePage {
    fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn on_install(&self) {
        self.go_next::<Next>();
    }

    #[template_callback]
    fn on_try(&self) {
        todo!("Start demo session");
    }
}

impl NextTarget for WelcomePage {
    fn page() -> BasePage {
        Self::new().upcast()
    }
}
