/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use gettextrs::gettext;

use super::confirm;
use super::wipe_or_free::WipeOrFreePage;
use crate::page::InstallPage;

glib::wrapper! {
    pub struct DiskSelectPage(ObjectSubclass<imp::DiskSelectPage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for DiskSelectPage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl DiskSelectPage {
    pub async fn page() -> BasePage {
        Self::default().upcast()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/disk/select.blp")]
    pub struct DiskSelectPage {
        #[template_child]
        header: TemplateChild<adw::StatusPage>,
        #[template_child]
        disks: TemplateChild<gtk::ListBox>,
        #[template_child]
        unusable_disks: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DiskSelectPage {
        const NAME: &'static str = "SetupDiskSelectPage";
        type Type = super::DiskSelectPage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DiskSelectPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.disks.bind_model(Some(&super::super::model::DisksModel::default()), |obj| {
                let disk = obj.downcast_ref::<super::super::obj::Disk>().unwrap();
                super::super::disk_row::DiskRow::new(disk).upcast()
            });

            // TODO: Connect to the changed signals of the listmodels
            self.on_changed();
            self.on_unusable_changed();

            // TODO: Enumerate the disks
        }
    }

    impl WidgetImpl for DiskSelectPage {}
    impl NavigationPageImpl for DiskSelectPage {}
    impl BasePageImpl for DiskSelectPage {}

    #[gtk::template_callbacks]
    impl DiskSelectPage {
        fn on_changed(&self) {
            if true {
                self.header.set_icon_name(None);
                self.header.set_title(&gettext("Select a Disk"));
                self.header.set_description(Some(&gettext(
                    "Select the disk to install the operating system to. <b>Make sure any important documents and files have been backed up.</b>"
                )));
                self.disks.set_visible(true);
            } else {
                self.header.set_icon_name(Some("drive-missing-symbolic"));
                self.header.set_title(&gettext("No Usable Disks Detected"));
                self.header.set_description(Some(&gettext(
                    "Setup could not detect any disks that are suitable to install the operating system to. Make sure that all disks are properly connected and meet the minimum requirements for Setup, and that the device's firmware is configured correctly."
                )));
                self.disks.set_visible(false);
            }
        }

        fn on_unusable_changed(&self) {
            self.unusable_disks.set_visible(true); // TODO
        }

        #[template_callback]
        async fn data_detected(&self) {
            self.obj().go_next(WipeOrFreePage::page());
        }

        #[template_callback]
        async fn nondestructive(&self) {
            if confirm::nondestructive(&*self.obj()).await {
                self.obj().go_next(InstallPage::page());
            }
        }

        #[template_callback]
        async fn full_wipe(&self) {
            if confirm::full_wipe(&*self.obj()).await {
                self.obj().go_next(InstallPage::page());
            }
        }

        #[template_callback]
        async fn insufficient_wipe(&self) {
            if confirm::insufficient_wipe(&*self.obj()).await {
                self.obj().go_next(InstallPage::page());
            }
        }

        #[template_callback]
        async fn confirm(&self) {
            todo!();
        }

        #[template_callback]
        fn power_off() {
            todo!();
        }
    }
}

