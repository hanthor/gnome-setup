/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use crate::page::{self, BasePageImpl, BasePageExt};
use crate::{window,osinfo};

use gettextrs::gettext;

pub mod done;

glib::wrapper! {
    pub struct InstallPage(ObjectSubclass<imp::InstallPage>)
        @extends gtk::Widget, adw::NavigationPage, page::BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for InstallPage {
        const NAME: &'static str = "SetupInstallPage";
        type Type = super::InstallPage;
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
    #[template(file = "src/page/install/install.blp")]
    pub struct InstallPage {
        #[template_child]
        info: TemplateChild<adw::StatusPage>,

        #[template_child]
        progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        status: TemplateChild<gtk::Label>,
    }

    impl ObjectImpl for InstallPage {
        fn constructed(&self) {
            self.parent_constructed();

            osinfo::bind_logo(self.info.get(), "icon-name");
            let title = &gettext!("Installing {}", osinfo::name());
            self.obj().set_title(title);
            self.info.set_title(title);

            // Mockup Stuff
            self.progress.set_fraction(0.5);
            self.status.set_label("Copying (50% complete)");
            let obj = self.obj();
            glib::timeout_add_local(
                std::time::Duration::new(3, 0),
                glib::clone!(@strong obj => move || {
                    obj.go_next(page::install::done::DonePage::new());
                    glib::ControlFlow::Break
                })
            );
        }
    }

    impl WidgetImpl for InstallPage {}
    impl NavigationPageImpl for InstallPage {}
    impl BasePageImpl for InstallPage {}
}

#[gtk::template_callbacks]
impl InstallPage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
