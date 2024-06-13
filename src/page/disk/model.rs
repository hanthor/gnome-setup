/* SPDX-License-Identifier: GPL-3.0-or-later */

use futures::prelude::*;
use gio::prelude::*;
use gio::subclass::prelude::*;

use std::cell::OnceCell;
use std::cmp::Ordering;
use std::collections::HashMap;

use gtk::gio;
use zbus::zvariant::OwnedObjectPath;

use super::obj::Disk;

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

        // TODO client: OnceCell<udisks2::Client>,
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

            let obj = self.obj();

            // Pass through the items changed signal
            self.store
                .connect_items_changed(glib::clone!(@strong obj => move |_, pos, rm, add| {
                    obj.items_changed(pos, rm, add)
                }));

            // Populate the list in the background
            glib::spawn_future_local(glib::clone!(@strong obj => async move {
                obj.imp().populate().await
            }));
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
            let conn = crate::system_bus().clone();
            let Ok(client) = udisks2::Client::new_for_connection(conn).await else {
                return log::error!("Failed construct udisks2 client");
            };

            self.repopulate(&client).await;

            let Ok(mut notifications) = client.object_manager().inner().receive_all_signals().await else {
                return log::error!("Failed to listen for changes from udisks2");
            };
            while let Some(msg) = notifications.next().await {
                // TODO: Debounce somehow?
                // TODO: Only if disks added / removed
                self.repopulate(&client).await;
            }
            log::warn!("udisks2 stopped sending disk notifications");
        }

        async fn repopulate(&self, client: &udisks2::Client) {
            self.store.remove_all();

            let objects = client.object_manager().get_managed_objects().await.unwrap_or_else(|_| {
                log::warn!("Failed to enumerate disks from udisks2 object manager");
                HashMap::new()
            });

            let root_drive = root_device(&client, objects.keys().cloned()).await;

            for path in objects.into_keys() {
                if root_drive.as_deref().is_some_and(|r| *r == *path) { continue; }

                // TODO: Unwrap w/ into_ok: https://github.com/rust-lang/rust/issues/61695
                let obj = client.object(path).unwrap();
                let Ok(drive) = obj.drive().await else { continue; };

                if drive.optical().await.unwrap_or(false) {
                    continue; // We don't support writing to optical media
                }

                if drive.size().await.unwrap_or(0) == 0 {
                    continue; // It's an empty SD card slot
                }

                // TODO: unwrap
                let drive = Disk::new(&client, drive).await.unwrap();
                self.store.append(&drive);
            }

            self.store.sort(cmp);
        }
    }
}

async fn root_device<I>(client: &udisks2::Client, objects: I) -> Option<OwnedObjectPath>
where
    I: Iterator<Item = OwnedObjectPath>
{
    for path in objects {
        // TODO: Unwrap w/ into_ok: https://github.com/rust-lang/rust/issues/61695
        let obj = client.object(path).unwrap();

        let Ok(fs) = obj.filesystem().await else { continue; };
        let Ok(mount_points) = fs.mount_points().await else { continue; };

        if !mount_points
            .into_iter()
            .filter_map(|mp| String::from_utf8(mp).ok())
            .any(|mp| mp == "/usr\0")
        {
            continue;
        }

        let Ok(block) = obj.block().await else { continue; };
        let Ok(drive) = block.drive().await else { continue; };
        return Some(drive);
    }

    None
}

fn cmp(a: &glib::Object, b: &glib::Object) -> Ordering {
    let a: &Disk = a.downcast_ref().unwrap();
    let b: &Disk = b.downcast_ref().unwrap();
    a.sort_key().cmp(&b.sort_key())
}
