/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::prelude::*;

use std::collections::HashMap;

use super::confirm;
use super::no_disks::NoDisksPage;
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
        // TODO: No unwraps on the client connection
        // TODO: Check if there are any disks
        let client =
            udisks2::Client::new_for_connection(crate::system_bus().clone()).await.unwrap();
        let objects = client.object_manager().get_managed_objects().await.unwrap();

        let root_drive = stream::iter(objects.keys().cloned())
            .filter_map(|path| {
                Box::pin(async {
                    // TODO: Cleanup once we have async blocks
                    let obj = client.object(path).ok()?;
                    let fs = obj.filesystem().await.ok()?;
                    let mount_points = fs.mount_points().await.ok()?;

                    if !mount_points
                        .iter()
                        .filter_map(|mount_point| std::str::from_utf8(mount_point.as_slice()).ok())
                        .any(|mount_point| mount_point == "/usr\0")
                    {
                        return None;
                    }

                    let block = obj.block().await.ok()?;
                    block.drive().await.ok()
                })
            })
            .next()
            .await;
        let root_drive = root_drive.as_deref();

        let drives: Vec<_> = stream::iter(objects.into_keys())
            .filter_map(|path| async {
                // https://github.com/rust-lang/futures-rs/issues/2464
                if root_drive.is_some_and(|r| *r == *path) {
                    return None;
                }

                let obj = client.object(path).ok()?;
                let drive = obj.drive().await.ok()?;

                if drive.optical().await.unwrap_or(false) {
                    return None; // We can't/shouldn't write to optical media
                }

                if drive.size().await.unwrap_or(0) == 0 {
                    return None; // It's an empty SD card slot
                }

                Some(drive)
            })
            .collect()
            .await;

        //let devices = client.manager().get_block_devices(HashMap::new()).await.unwrap();
        // TODO: Use Result.into_ok https://github.com/rust-lang/rust/issues/61695
        // https://github.com/rust-lang/futures-rs/issues/2464
        //dbg!(&drives);

        if drives.is_empty() {
            NoDisksPage::page().await
        } else {
            Self::default().upcast()
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/disk/select.blp")]
    pub struct DiskSelectPage {
        #[template_child]
        disks: TemplateChild<gtk::ListBox>,
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

            // TODO: Enumerate the disks
        }
    }

    impl WidgetImpl for DiskSelectPage {}
    impl NavigationPageImpl for DiskSelectPage {}
    impl BasePageImpl for DiskSelectPage {}

    #[gtk::template_callbacks]
    impl DiskSelectPage {
        #[template_callback]
        async fn confirm(&self) {
            match self.disks.selected_row().unwrap().index() {
                0 => {
                    self.obj().go_next(WipeOrFreePage::page());
                }
                1 => {
                    if confirm::nondestructive(&*self.obj()).await {
                        self.obj().go_next(InstallPage::page());
                    }
                }
                2 => {
                    if confirm::full_wipe(&*self.obj()).await {
                        self.obj().go_next(InstallPage::page());
                    }
                }
                3 => {
                    if confirm::insufficient_wipe(&*self.obj()).await {
                        self.obj().go_next(InstallPage::page());
                    }
                }
                _ => (),
            }
        }
    }
}
