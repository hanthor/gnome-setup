/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use gettextrs::gettext;
use gtk::glib;

use crate::osinfo;
use crate::page::InstallPage as Next;

glib::wrapper! {
    pub struct WelcomePage(ObjectSubclass<imp::WelcomePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for WelcomePage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl NextTarget for WelcomePage {
    fn page() -> BasePage {
        Self::default().upcast()
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
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WelcomePage {
        fn constructed(&self) {
            self.parent_constructed();

            osinfo::bind_logo(self.info.get(), "icon-name");

            if crate::in_session() {
                self.obj().set_title(&gettext("Welcome"));
                self.info.set_title(&gettext("Ready to Install?"));
                self.info.set_description(Some(&gettext!(
                    "You can start an installation while you continue testing {}",
                    osinfo::name(),
                )));
                self.try_btn.set_visible(false);
            } else {
                self.obj().set_title(&gettext("Try or Install?"));
                self.info.set_title(&gettext!("Welcome to {}!", osinfo::name()));
                self.info.set_description(Some(&gettext!(
                    "You can try {} in a temporary environment before you install, or \
                    you can start an installation right away.",
                    osinfo::name(),
                )));
                self.try_btn.set_label(&gettext!("_Try {}", osinfo::name()));
            }
        }
    }

    impl WidgetImpl for WelcomePage {}
    impl NavigationPageImpl for WelcomePage {}
    impl BasePageImpl for WelcomePage {}

    #[gtk::template_callbacks]
    impl WelcomePage {
        #[template_callback]
        fn on_install(&self) {
            self.obj().go_next::<Next>();
        }

        #[template_callback]
        fn on_try(&self) {
            todo!("Start demo session");
        }
    }
}
