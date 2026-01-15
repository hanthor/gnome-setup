/* SPDX-License-Identifier: GPL-3.0-or-later */

use log::error;

use gio::prelude::*;
use gio::subclass::prelude::*;

use gtk::gio;

use crate::models::Disk;

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

    #[derive(Debug)]
    pub struct DisksModel {
        store: gio::ListStore,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DisksModel {
        const NAME: &'static str = "SetupDisksModel";
        type Type = super::DisksModel;
        type Interfaces = (gio::ListModel,);

        fn new() -> Self {
            Self {
                store: gio::ListStore::new::<Disk>(),
            }
        }
    }

    impl ObjectImpl for DisksModel {
        fn constructed(&self) {
            self.parent_constructed();

            // Pass through the items changed signal
            self.store.connect_items_changed(glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |_, pos, rm, add| this.obj().items_changed(pos, rm, add)
            ));

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
            self.store.n_items()
        }

        fn item(&self, pos: u32) -> Option<glib::Object> {
            self.store.item(pos)
        }
    }

    impl DisksModel {
        async fn populate(&self) {
            // FIXME: we don't get devices changes from repart
            let repart = crate::interfaces::Repart::new();
            let devices = match repart.candidate_devices().await {
                Ok(devices) => devices,
                Err(error) => {
                    error!("Failed to talk to systemd-repart: {error}");
                    return;
                }
            };

            for disk in devices {
                self.store.append(&disk);
            }
        }
    }
}
