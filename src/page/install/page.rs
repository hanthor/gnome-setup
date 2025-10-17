/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use formatx::formatx;
use gettextrs::gettext;
use gtk::glib;

use super::done_page::DonePage;
use crate::osinfo;

glib::wrapper! {
    pub struct InstallPage(ObjectSubclass<imp::InstallPage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for InstallPage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl InstallPage {
    pub fn page() -> BasePage {
        Self::default().upcast()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/install/page.blp")]
    pub struct InstallPage {
        #[template_child]
        info: TemplateChild<adw::StatusPage>,

        #[template_child]
        progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        status: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InstallPage {
        const NAME: &'static str = "SetupInstallPage";
        type Type = super::InstallPage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for InstallPage {
        fn constructed(&self) {
            self.parent_constructed();

            osinfo::bind_logo(self.info.get(), "icon-name");
            let title = formatx!(gettext("Installing {}"), osinfo::name()).unwrap();
            self.obj().set_title(&title);
            self.info.set_title(&title);

            // Mockup Stuff
            self.progress.set_fraction(0.5);
            self.status.set_label("Copying (50% complete)");
            let obj = self.obj();
            glib::timeout_add_local(
                std::time::Duration::new(3, 0),
                glib::clone!(
                    #[strong]
                    obj,
                    move || {
                        obj.go_next(DonePage::page());
                        glib::ControlFlow::Break
                    }
                ),
            );
        }
    }

    impl WidgetImpl for InstallPage {}
    impl NavigationPageImpl for InstallPage {}
    impl BasePageImpl for InstallPage {}
}
