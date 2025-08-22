/* SPDX-License-Identifier: GPL-3.0-or-later */

use glib::prelude::*;
use glib::subclass::prelude::*;

use std::cell::{Cell, OnceCell};
use std::marker::PhantomData;

type DriveProxy = udisks2::drive::DriveProxy<'static>;

glib::wrapper! {
    pub struct Disk(ObjectSubclass<imp::Disk>);
}

impl Disk {
    pub async fn new(client: &udisks2::Client, drive: DriveProxy) -> Result<Self, anyhow::Error> {
        // TODO: Figure out error handling

        let total_size = drive.size().await?;
        let obj: Self = glib::Object::builder()
            .property("vendor", drive.vendor().await?)
            .property("model", drive.model().await?)
            .property("sort-key", drive.sort_key().await?)
            .property("total-size", total_size)
            .build();
        let status = &obj.imp().status;

        // TODO: Figure out error handling
        let block = client.block_for_drive(&drive, false).await.ok_or(anyhow::anyhow!("TODO"))?;
        // TODO: Unwrap w/ into_ok: https://github.com/rust-lang/rust/issues/61695
        let ptable = client.object(block.inner().path().clone()).unwrap().partition_table().await?;

        if ptable.type_().await? != "gpt" {
            status.set(DiskStatus::UnsupportedPartitionTable);
            return Ok(obj);
        };

        let partitions = client.partitions(&ptable).await;

        if partitions.is_empty() {
            status.set(DiskStatus::Empty);
            return Ok(obj);
        }

        let mut used_space = 0u64;

        //partitions.sort_by_key(|p| p.offset().await);
        let mut part_iter = partitions.into_iter().peekable();
        while let Some(part) = part_iter.next() {
            let part_size = part.size().await?;
            used_space += part_size;

            // Ignore any slack, if there's less than 1MiB between this and the next partition
            if let Some(next) = part_iter.peek() {
                let part_end = part.offset().await? + part_size;
                let next_start = next.offset().await?;
                dbg!(part.offset().await, part_end, next_start);
                assert!(part_end < next_start);
                let slack = next_start - part_end;
                if slack < 1024*1024 {
                    used_space += slack;
                }
            }
        }
        assert!(used_space < total_size);
        if total_size - used_space < 1024*1024 {
            used_space = total_size;
        }

        let free_space = total_size - used_space;

        status.set(if total_size == used_space {
            DiskStatus::Full
        } else if free_space < 20*1024*1024*1024 { // TODO: No hard-coded size! Get from backend
            DiskStatus::InsufficientFreeSpace { free_space }
        } else {
            DiskStatus::DataDetected { free_space }
        });
        Ok(obj)
    }

    pub fn status(&self) -> DiskStatus {
        *self.imp().status.get().unwrap()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DiskStatus {
    Empty,
    DataDetected { free_space: u64 },
    InsufficientFreeSpace { free_space: u64 },
    Full,
    UnsupportedPartitionTable,
    TooSmall,
}

impl DiskStatus {
    pub fn usable(self) -> bool {
        match self {
            Self::Empty | Self::DataDetected {..} | Self::InsufficientFreeSpace {..} | Self::Full | Self::UnsupportedPartitionTable => true,
            Self::TooSmall => false,
        }
    }
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
        vendor: OnceCell<String>,
        #[property(get, construct_only)]
        model: OnceCell<String>,

        #[property(get, construct_only)]
        free_space: Cell<u64>,
        #[property(get, construct_only)]
        total_size: Cell<u64>,

        // https://github.com/gtk-rs/gtk-rs-core/issues/1108
        //#[property(get, construct_only, builder(DiskStatus::Full))]
        pub(super) status: OnceCell<DiskStatus>,

        #[property(get, construct_only)]
        sort_key: OnceCell<String>,

        #[property(get = Self::display_name)]
        display_name: PhantomData<String>,
        #[property(get = Self::display_desc)]
        display_desc: PhantomData<String>,
        #[property(get = Self::icon_name)]
        icon_name: PhantomData<String>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for Disk {}

    impl Disk {
        fn display_name(&self) -> String {
            let mut out = self.vendor.get().unwrap().clone();
            if !out.is_empty() { out.push(' '); }
            out.push_str(&self.model.get().unwrap());
            out.push(' ');
            out.push_str(&glib::format_size(self.total_size.get()));
            out
        }

        fn display_desc(&self) -> String {
            //todo!()
            "Empty — 16 GB Total".into()
        }

        fn icon_name(&self) -> String {
            //todo!()
            "drive-harddisk-usb-symbolic".into()
        }
    }
}
