/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use gettextrs::gettext;

use super::confirm;
use crate::osinfo;
use crate::page::InstallPage;

glib::wrapper! {
    pub struct WipeOrFreePage(ObjectSubclass<imp::WipeOrFreePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for WipeOrFreePage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl WipeOrFreePage {
    pub async fn page() -> BasePage {
        Self::default().upcast()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/disk/wipe_or_free.blp")]
    pub struct WipeOrFreePage {
        #[template_child]
        heading: TemplateChild<adw::StatusPage>,
        #[template_child]
        disk_info: TemplateChild<adw::ActionRow>,
        #[template_child]
        full_option: TemplateChild<adw::ActionRow>,
        #[template_child]
        free_option: TemplateChild<adw::ActionRow>,
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

    impl ObjectImpl for WipeOrFreePage {
        fn constructed(&self) {
            self.parent_constructed();

            let disk_name = "Sandmug SSD 960 1TB";
            let disk_summary = "128 GB Used / 768 GB Free";

            self.heading.set_description(Some(&gettext!(
                "Data detected on <b>{}</b>. Setup can use the full disk for installation, which will erase all software, documents, and data, or install into the free space, preserving existing data.",
                disk_name
            )));

            self.disk_info.set_title(disk_name);
            self.disk_info.set_subtitle(disk_summary);

            self.full_option.set_subtitle(&gettext!(
                "Erase all software, documents, and data from the disk and do a fresh installation of {}",
                osinfo::name()
            ));

            self.free_option.set_subtitle(&gettext!(
                "Keep the existing data, and install {} into the free space on disk",
                osinfo::name()
            ));
        }
    }

    impl WidgetImpl for WipeOrFreePage {}
    impl NavigationPageImpl for WipeOrFreePage {}
    impl BasePageImpl for WipeOrFreePage {}

    #[gtk::template_callbacks]
    impl WipeOrFreePage {
        #[template_callback]
        async fn use_full_disk(&self) {
            if confirm::expected_wipe(&*self.obj()).await {
                self.obj().go_next(InstallPage::page());
            }
        }

        #[template_callback]
        async fn use_free_space(&self) {
            if confirm::nondestructive(&*self.obj()).await {
                self.obj().go_next(InstallPage::page());
            }
        }
    }
}
