/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use formatx::formatx;
use gettextrs::gettext;

use crate::models::{Disk, DiskStatus};
use crate::osinfo;

glib::wrapper! {
    pub struct DiskRow(ObjectSubclass<imp::DiskRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl DiskRow {
    pub fn new(disk: &Disk) -> Self {
        glib::Object::builder().property("disk", disk).build()
    }
}

mod imp {
    use super::*;

    use std::cell::RefCell;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/disk/disk_row.blp")]
    #[properties(wrapper_type = super::DiskRow)]
    pub struct DiskRow {
        #[property(get, set = Self::set_disk, explicit_notify)]
        disk: RefCell<Option<Disk>>,
        disk_status_nofity_id: RefCell<Option<glib::SignalHandlerId>>,

        #[template_child]
        details_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        details_title: TemplateChild<gtk::Label>,
        #[template_child]
        details_desc: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DiskRow {
        const NAME: &'static str = "SetupDiskRow";
        type Type = super::DiskRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            Disk::static_type();
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DiskRow {
        fn dispose(&self) {
            if let Some(old_disk) = self.disk.take() {
                if let Some(disk_status_nofity_id) = self.disk_status_nofity_id.take() {
                    old_disk.disconnect(disk_status_nofity_id);
                }
            }
        }
    }

    impl DiskRow {
        fn set_disk(&self, disk: Option<Disk>) {
            let mut disk_cell = self.disk.borrow_mut();

            if *disk_cell == disk {
                return;
            }

            if let Some(ref old_disk) = *disk_cell {
                if let Some(disk_status_nofity_id) = self.disk_status_nofity_id.take() {
                    old_disk.disconnect(disk_status_nofity_id);
                }
            }

            *disk_cell = disk;

            if let Some(ref disk) = *disk_cell {
                let handle = disk.connect_status_notify(glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |_| {
                        this.update_details();
                    }
                ));

                self.disk_status_nofity_id.replace(Some(handle));
            }

            drop(disk_cell);
            self.update_details();
            self.obj().notify_disk();
        }

        fn update_details(&self) {
            let Some(ref disk) = *self.disk.borrow() else {
                self.details_button.set_visible(false);
                return;
            };

            match disk.status() {
                DiskStatus::Empty | DiskStatus::DataDetected => {}
                DiskStatus::InsufficientFreeSpace => {
                    self.details_title.set_label(&gettext("Disk Has Insufficient Free Space"));
                    self.details_desc.set_label(&gettext("This disk does not have enough free space for an installation. Setup will only offer to use the entire disk during installation."));
                }
                DiskStatus::Full => {
                    self.details_title.set_label(&gettext("Disk Is Full"));
                    self.details_desc.set_label(&gettext("This disk is full or has very limit free space available. Setup will only offer to use the entire disk during installation."));
                }
                DiskStatus::UnsupportedPartitionTable => {
                    self.details_title.set_label(&gettext("Disk Has Unsupported Partition Table"));
                    self.details_desc.set_label(&gettext("This disk has an unsupported partition table. Only GTP is supported by Setup."));
                }
                DiskStatus::TooSmall => {
                    self.details_title.set_label(&gettext("Disk Too Small"));
                    self.details_desc.set_label(&formatx!(gettext("This disk cannot be used by Setup to perform  an installation of {os_name} as it requires at least {min_disk_size} GB of available space."), os_name = osinfo::name(), min_disk_size = glib::format_size(disk.min_disk_size() as u64)).expect("Proper translated format string"));
                }
            }

            match disk.status() {
                DiskStatus::Empty | DiskStatus::DataDetected => {
                    self.details_button.set_visible(false);
                }
                DiskStatus::InsufficientFreeSpace
                | DiskStatus::Full
                | DiskStatus::UnsupportedPartitionTable => {
                    self.details_button.set_visible(true);
                    self.details_button.add_css_class("warning");
                    self.details_button.set_icon_name("dialog-warning-symbolic");
                    self.details_button.set_tooltip_text(Some(&gettext("Warning")));
                }
                DiskStatus::TooSmall => {
                    self.details_button.set_visible(true);
                    self.details_button.remove_css_class("warning");
                    self.details_button.set_icon_name("help-about-symbolic");
                    self.details_button.set_tooltip_text(Some(&gettext("More Information")));
                }
            }
        }
    }

    impl WidgetImpl for DiskRow {}
    impl ListBoxRowImpl for DiskRow {}
    impl PreferencesRowImpl for DiskRow {}
    impl ActionRowImpl for DiskRow {}
}
