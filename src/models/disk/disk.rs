/* SPDX-License-Identifier: GPL-3.0-or-later */

use gettextrs::gettext;
use glib::prelude::*;
use glib::subclass::prelude::*;

use std::cell::{Cell, OnceCell};
use std::marker::PhantomData;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "SetupDiskStatus")]
pub enum DiskStatus {
    #[default]
    Empty,
    DataDetected,
    InsufficientFreeSpace,
    Full,
    UnsupportedPartitionTable,
    TooSmall,
}

impl DiskStatus {
    pub fn usable(self) -> bool {
        match self {
            Self::Empty
            | Self::DataDetected
            | Self::InsufficientFreeSpace
            | Self::Full
            | Self::UnsupportedPartitionTable => true,
            Self::TooSmall => false,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "SetupInstallPhase")]
pub enum InstallPhase {
    #[default]
    None,
    WarmUp,
    Minimizing,
    Placing,
    WipingDisk,
    WipingPartition,
    CopyingPartition,
    FormattingPartition,
    AdjustingPartition,
    WritingTable,
    InstallBootloader,
    Finished,
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for Disk {
        const NAME: &'static str = "SetupDisk";
        type Type = super::Disk;
    }

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Disk)]
    pub struct Disk {
        #[property(get, construct_only)]
        node: OnceCell<String>,
        #[property(get = Self::subsystem, set = Self::set_subsystem, construct_only, nullable, type = Option<String>)]
        subsystem: OnceCell<String>,
        #[property(get = Self::vendor, set = Self::set_vendor, construct_only, nullable, type = Option<String>)]
        vendor: OnceCell<String>,
        #[property(get = Self::model, set = Self::set_model, construct_only, nullable, type = Option<String>)]
        model: OnceCell<String>,

        #[property(get, construct_only)]
        size: Cell<i64>,
        #[property(get, set)]
        min_disk_size: Cell<i64>,

        #[property(get, set, builder(DiskStatus::Empty))]
        status: Cell<DiskStatus>,
        #[property(get, set, builder(InstallPhase::None))]
        install_phase: Cell<InstallPhase>,
        #[property(get, set)]
        install_progress: Cell<i64>,

        #[property(get = Self::display_name)]
        display_name: PhantomData<String>,
        #[property(get = Self::display_desc)]
        display_desc: PhantomData<String>,
        #[property(get = Self::icon_name)]
        icon_name: PhantomData<String>,
        #[property(get = |this: &&Self| this.status.get().usable())]
        usable: PhantomData<bool>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for Disk {}

    impl Disk {
        fn subsystem(&self) -> Option<String> {
            self.subsystem.get().cloned()
        }

        fn set_subsystem(&self, subsystem: Option<String>) {
            if let Some(subsystem) = subsystem {
                self.subsystem.set(subsystem).unwrap();
            }
        }

        fn vendor(&self) -> Option<String> {
            self.vendor.get().cloned()
        }

        fn set_vendor(&self, vendor: Option<String>) {
            if let Some(vendor) = vendor {
                self.vendor.set(vendor).unwrap();
            }
        }

        fn model(&self) -> Option<String> {
            self.model.get().cloned()
        }

        fn set_model(&self, model: Option<String>) {
            if let Some(model) = model {
                self.model.set(model).unwrap();
            }
        }

        fn display_name(&self) -> String {
            let vendor = self.obj().vendor();
            let model = self.obj().model();

            let mut out = if let Some(mut vendor) = vendor {
                if let Some(model) = model {
                    vendor.push(' ');
                    vendor.push_str(&model);
                }
                vendor
            } else if let Some(model) = model {
                model
            } else {
                gettext("Unknown Disk")
            };

            // FIXME: the model might already have the total size, this happens on my disk
            out.push(' ');
            out.push_str(&glib::format_size(self.obj().size() as u64));

            out
        }

        fn display_desc(&self) -> String {
            if self.status.get() == DiskStatus::Empty {
                gettext("This disk is fully empty")
            } else {
                gettext("This disk contains existing data")
            }
        }

        fn icon_name(&self) -> String {
            //todo!()
            "drive-harddisk-usb-symbolic".into()
        }
    }
}

glib::wrapper! {
    pub struct Disk(ObjectSubclass<imp::Disk>);
}

impl Disk {
    pub async fn install(&self, erase: bool) -> anyhow::Result<()> {
        let sysinstall = crate::interfaces::SysInstall::new();
        sysinstall.install(self, erase).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: i64 = 1_000_000_000;

    fn disk(vendor: Option<&str>, model: Option<&str>, size: i64) -> Disk {
        let mut builder = glib::Object::builder::<Disk>()
            .property("node", "/dev/test")
            .property("size", size);
        if let Some(vendor) = vendor {
            builder = builder.property("vendor", vendor);
        }
        if let Some(model) = model {
            builder = builder.property("model", model);
        }
        builder.build()
    }

    fn size_str(size: i64) -> String {
        glib::format_size(size as u64).to_string()
    }

    #[test]
    fn only_too_small_is_unusable() {
        let usable = [
            DiskStatus::Empty,
            DiskStatus::DataDetected,
            DiskStatus::InsufficientFreeSpace,
            DiskStatus::Full,
            DiskStatus::UnsupportedPartitionTable,
        ];
        for status in usable {
            assert!(status.usable(), "{status:?} should be usable");
        }
        assert!(!DiskStatus::TooSmall.usable());
    }

    #[test]
    fn usable_property_follows_status() {
        let disk = disk(None, None, GB);
        assert!(disk.usable());

        disk.set_status(DiskStatus::TooSmall);
        assert!(!disk.usable());

        disk.set_status(DiskStatus::Full);
        assert!(disk.usable());
    }

    #[test]
    fn display_name_joins_vendor_model_and_size() {
        let disk = disk(Some("ACME"), Some("Speedy"), GB);
        assert_eq!(disk.display_name(), format!("ACME Speedy {}", size_str(GB)));
    }

    #[test]
    fn display_name_with_vendor_only() {
        let disk = disk(Some("ACME"), None, GB);
        assert_eq!(disk.display_name(), format!("ACME {}", size_str(GB)));
    }

    #[test]
    fn display_name_with_model_only() {
        let disk = disk(None, Some("Speedy"), GB);
        assert_eq!(disk.display_name(), format!("Speedy {}", size_str(GB)));
    }

    #[test]
    fn display_name_falls_back_to_unknown_disk() {
        let disk = disk(None, None, GB);
        assert_eq!(
            disk.display_name(),
            format!("{} {}", gettext("Unknown Disk"), size_str(GB))
        );
    }

    #[test]
    fn display_desc_distinguishes_empty_disks() {
        let disk = disk(None, None, GB);
        assert_eq!(disk.display_desc(), gettext("This disk is fully empty"));

        disk.set_status(DiskStatus::DataDetected);
        assert_eq!(
            disk.display_desc(),
            gettext("This disk contains existing data")
        );
    }
}
