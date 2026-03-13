/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;

use super::select::DiskSelectPage as Next;
use crate::page::base::{BasePage, BasePageExt, BasePageImpl};

glib::wrapper! {
    pub struct RstPage(ObjectSubclass<imp::RstPage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for RstPage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl RstPage {
    pub fn page() -> BasePage {
        // Adapted from https://github.com/canonical/subiquity/blob/3b8f018/subiquity/server/controllers/filesystem.py#L1000-L1006
        let using_rst = glob::glob("/sys/module/ahci/drivers/pci:ahci/*/remapped_nvme")
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .filter_map(|num_remapped| num_remapped.parse::<usize>().ok())
            .any(|num_remapped| num_remapped > 0);

        if using_rst {
            Self::default().upcast()
        } else {
            Next::page()
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/disk/rst.blp")]
    pub struct RstPage;

    #[glib::object_subclass]
    impl ObjectSubclass for RstPage {
        const NAME: &'static str = "SetupRstPage";
        type Type = super::RstPage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RstPage {}
    impl WidgetImpl for RstPage {}
    impl NavigationPageImpl for RstPage {}
    impl BasePageImpl for RstPage {}

    #[gtk::template_callbacks]
    impl RstPage {
        #[template_callback]
        fn next(&self) {
            self.obj().go_subpage(Next::page());
        }
    }
}
