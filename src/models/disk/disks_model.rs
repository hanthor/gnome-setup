/* SPDX-License-Identifier: GPL-3.0-or-later */

use std::cell::RefCell;

use indexmap::IndexMap;
use log::error;

use gio::prelude::*;
use gio::subclass::prelude::*;

use gtk::gio;

use super::disk::Disk;

glib::wrapper! {
    pub struct DisksModel(ObjectSubclass<imp::DisksModel>)
        @implements gio::ListModel;
}

impl Default for DisksModel {
    fn default() -> Self {
        thread_local!(static ONCE: DisksModel = glib::Object::new());
        ONCE.with(|v| v.clone())
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct DisksModel {
        pub(super) disks: RefCell<IndexMap<String, Disk>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DisksModel {
        const NAME: &'static str = "SetupDisksModel";
        type Type = super::DisksModel;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for DisksModel {
        fn constructed(&self) {
            self.parent_constructed();

            // Populate the list in the background
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = this)]
                self,
                async move { this.populate().await }
            ));
        }
    }

    impl ListModelImpl for DisksModel {
        fn item_type(&self) -> glib::Type {
            Disk::static_type()
        }

        fn n_items(&self) -> u32 {
            self.disks.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.disks
                .borrow()
                .get_index(position as usize)
                .map(|(_node, disk)| disk.clone().upcast())
        }
    }

    impl DisksModel {
        async fn populate(&self) {
            let sysinstall = crate::interfaces::SysInstall::new();
            let result = sysinstall
                .candidate_devices(
                    glib::clone!(
                        #[weak(rename_to = this)]
                        self,
                        move |disk| {
                            let index = this
                                .disks
                                .borrow_mut()
                                .entry(disk.node())
                                .insert_entry(disk)
                                .index();
                            this.obj().items_changed(index as u32, 0, 1)
                        }
                    ),
                    glib::clone!(
                        #[weak(rename_to = this)]
                        self,
                        move |node| {
                            let Some((index, _, _)) =
                                this.disks.borrow_mut().shift_remove_full(node)
                            else {
                                return;
                            };
                            this.obj().items_changed(index as u32, 1, 0)
                        }
                    ),
                )
                .await;
            if let Err(error) = result {
                error!("Failed to fetch candidate devices {error:?}");
            }
        }
    }
}
