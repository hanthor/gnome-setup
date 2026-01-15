/* SPDX-License-Identifier: GPL-3.0-or-later */

use gettextrs::gettext;
use glib::prelude::*;
use glib::subclass::prelude::*;
use log::error;

use std::cell::{Cell, OnceCell};
use std::marker::PhantomData;

use crate::interfaces::CandidateDevice;

glib::wrapper! {
    pub struct Disk(ObjectSubclass<imp::Disk>);
}

impl Disk {
    pub fn from_candidate_device(device: &CandidateDevice) -> Self {
        let CandidateDevice {
            node,
            subsystem,
            vendor,
            model,
            size_bytes,
            ..
        } = device;
        glib::Object::builder()
            .property("node", node)
            .property("subsystem", subsystem.as_ref().filter(|s| !s.is_empty()))
            .property("vendor", vendor.as_ref().filter(|s| !s.is_empty()))
            .property("model", model.as_ref().filter(|s| !s.is_empty()))
            .property("size", size_bytes.unwrap_or_default())
            .build()
    }

    pub fn demo() -> Self {
        glib::Object::builder().property("node", "some device demo").build()
    }

    pub async fn install(&self, erase: bool) {
        let repart = crate::interfaces::Repart::new();
        if let Err(error) = repart.install(self, erase).await {
            error!("Failed to install {error}");
        }
    }
}

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
