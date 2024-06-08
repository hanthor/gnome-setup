/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

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
        // TODO: Check if there are any disks
        //NoDisksPage::page()

        //let foo = NoDisksPage::page();
        glib::timeout_future_seconds(5).await;

        Self::default().upcast()
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
