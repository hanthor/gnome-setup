/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::base::{BasePage, BasePageExt, BasePageImpl};
use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::clone;

use gettextrs::gettext;

use super::confirm;
use super::disk_row::DiskRow;
use super::wipe_or_free::WipeOrFreePage;
use crate::models::{Disk, DiskStatus, DisksModel};
use crate::osinfo;
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
    pub fn page() -> BasePage {
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
        #[template_child]
        power_off_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DiskSelectPage {
        const NAME: &'static str = "SetupDiskSelectPage";
        type Type = super::DiskSelectPage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            osinfo::OsInfoCallbacks::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DiskSelectPage {
        fn constructed(&self) {
            self.parent_constructed();

            let disks_model = DisksModel::default();

            disks_model.connect_closure(
                "items-changed",
                true,
                glib::closure_local!(
                    #[watch(rename_to = obj)]
                    self.obj(),
                    move |_: glib::Object, _: u32, _: u32, _: u32| {
                        obj.imp().on_disks_changed();
                    }
                ),
            );

            let usable_filter =
                gtk::BoolFilter::builder().expression(Disk::this_expression("usable")).build();
            let usable_disks =
                gtk::FilterListModel::new(Some(disks_model.clone()), Some(usable_filter));
            self.disks.bind_model(
                Some(&usable_disks),
                clone!(
                    #[weak(rename_to = this)]
                    self,
                    #[upgrade_or_panic]
                    move |obj| {
                        let disk = obj.downcast_ref::<Disk>().unwrap();
                        let row = DiskRow::new(disk);
                        row.connect_activated(clone!(
                            #[weak]
                            this,
                            move |row| {
                                let disk: Disk =
                                    row.disk().expect("Every row has a associated disk");
                                this.activate_disk(disk);
                            }
                        ));
                        row.upcast()
                    }
                ),
            );

            let unusable_filter = gtk::BoolFilter::builder()
                .invert(true)
                .expression(Disk::this_expression("usable"))
                .build();
            let unusable_disks =
                gtk::FilterListModel::new(Some(disks_model), Some(unusable_filter));
            self.unusable_disks.bind_model(Some(&unusable_disks), |obj| {
                let disk = obj.downcast_ref::<Disk>().unwrap();
                let row = DiskRow::new(disk);
                row.set_activatable(false);
                row.upcast()
            });

            self.on_disks_changed();
        }
    }

    impl WidgetImpl for DiskSelectPage {}
    impl NavigationPageImpl for DiskSelectPage {}
    impl BasePageImpl for DiskSelectPage {}

    #[gtk::template_callbacks]
    impl DiskSelectPage {
        fn on_disks_changed(&self) {
            let has_usable_disks = self.disks.row_at_index(0).is_some();
            let has_unusable_disks = self.unusable_disks.row_at_index(0).is_some();

            self.disks.set_visible(has_usable_disks);
            self.unusable_disks.set_visible(has_unusable_disks);

            if has_usable_disks || has_unusable_disks {
                self.header.set_icon_name(None);
                self.header.set_title(&gettext("Select a Disk"));
                self.header.set_description(Some(&gettext(
                    "Select the disk to install the operating system to. <b>Make sure any important documents and files have been backed up.</b>"
                )));
                self.power_off_button.set_visible(false);
            } else {
                self.header.set_icon_name(Some("drive-missing-symbolic"));
                self.header.set_title(&gettext("No Usable Disks Detected"));
                self.header.set_description(Some(&gettext(
                    "Setup could not detect any disks that are suitable to install the operating system to. Make sure that all disks are properly connected and meet the minimum requirements for Setup, and that the device's firmware is configured correctly."
                )));
                self.power_off_button.set_visible(true);
            }
        }

        fn activate_disk(&self, disk: Disk) {
            match disk.status() {
                DiskStatus::Empty => {
                    self.nondestructive(disk);
                }
                DiskStatus::DataDetected => {
                    self.data_detected(disk);
                }
                DiskStatus::InsufficientFreeSpace | DiskStatus::Full => {
                    self.insufficient_wipe(disk);
                }
                DiskStatus::UnsupportedPartitionTable => {
                    self.full_wipe(disk);
                }
                DiskStatus::TooSmall => {
                    unreachable!("Disks that are to small can't be activated");
                }
            }
        }

        fn data_detected(&self, disk: Disk) {
            self.obj().go_subpage(WipeOrFreePage::page(disk));
        }

        fn nondestructive(&self, disk: Disk) {
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                async move {
                    if confirm::nondestructive(&obj).await {
                        obj.go_subpage(InstallPage::page(disk).await);
                    }
                }
            ));
        }

        fn full_wipe(&self, disk: Disk) {
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                async move {
                    if confirm::full_wipe(&obj).await {
                        obj.go_subpage(InstallPage::page(disk).await);
                    }
                }
            ));
        }

        fn insufficient_wipe(&self, disk: Disk) {
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                async move {
                    if confirm::insufficient_wipe(&obj).await {
                        obj.go_subpage(InstallPage::page(disk).await);
                    }
                }
            ));
        }

        #[template_callback]
        fn power_off() {
            todo!();
        }
    }
}
