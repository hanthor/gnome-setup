/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

glib::wrapper! {
    pub struct NoDisksPage(ObjectSubclass<imp::NoDisksPage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for NoDisksPage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl NoDisksPage {
    pub async fn page() -> BasePage {
        Self::default().upcast()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/disk/no_disks.blp")]
    pub struct NoDisksPage;

    #[glib::object_subclass]
    impl ObjectSubclass for NoDisksPage {
        const NAME: &'static str = "SetupNoDisksPage";
        type Type = super::NoDisksPage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NoDisksPage {}
    impl WidgetImpl for NoDisksPage {}
    impl NavigationPageImpl for NoDisksPage {}
    impl BasePageImpl for NoDisksPage {}

    #[gtk::template_callbacks]
    impl NoDisksPage {
        #[template_callback]
        fn power_off() {
            todo!();
        }
    }
}
