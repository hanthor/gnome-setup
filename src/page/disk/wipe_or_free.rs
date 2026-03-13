/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::base::{BasePage, BasePageExt, BasePageImpl};
use adw::prelude::*;
use adw::subclass::prelude::*;

use formatx::formatx;

use super::confirm;
use crate::models::Disk;
use crate::page::InstallPage;

glib::wrapper! {
    pub struct WipeOrFreePage(ObjectSubclass<imp::WipeOrFreePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl WipeOrFreePage {
    pub fn new(disk: &Disk) -> Self {
        glib::Object::builder().property("disk", disk).build()
    }

    pub fn page(disk: Disk) -> BasePage {
        Self::new(&disk).upcast()
    }
}

mod imp {
    use super::*;

    use std::cell::RefCell;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/disk/wipe_or_free.blp")]
    #[properties(wrapper_type = super::WipeOrFreePage)]
    pub struct WipeOrFreePage {
        #[property(get, set)]
        disk: RefCell<Option<Disk>>,

        #[template_child]
        heading: TemplateChild<adw::StatusPage>,
        #[template_child]
        free_radio: TemplateChild<gtk::CheckButton>,
        #[template_child]
        go_btn: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WipeOrFreePage {
        const NAME: &'static str = "SetupWipeOrFreePage";
        type Type = super::WipeOrFreePage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for WipeOrFreePage {}
    impl WidgetImpl for WipeOrFreePage {}
    impl NavigationPageImpl for WipeOrFreePage {}
    impl BasePageImpl for WipeOrFreePage {}

    #[gtk::template_callbacks]
    impl WipeOrFreePage {
        #[template_callback]
        async fn install(&self) {
            let disk = self.obj().disk().expect("Every row has a associated disk");

            if !self.free_radio.is_active() && !confirm::expected_wipe(&*self.obj()).await {
                return;
            }

            self.obj().go_subpage(InstallPage::page(disk).await);
        }

        #[template_callback(function)]
        fn apply_disk_name(disk_name: &str, string: &str) -> String {
            formatx!(string, disk_name = disk_name,).unwrap()
        }
    }
}
