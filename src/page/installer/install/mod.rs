/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::base::{BasePage, BasePageExt, BasePageImpl};
use adw::prelude::*;
use adw::subclass::prelude::*;
use formatx::formatx;
use gettextrs::gettext;
use gtk::glib;
use log::error;
use std::cell::RefCell;

use crate::models::disk::{Disk, InstallPhase};
use crate::osinfo;

glib::wrapper! {
    pub struct InstallPage(ObjectSubclass<imp::InstallPage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for InstallPage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl InstallPage {
    pub fn new(disk: &Disk, wipe: bool) -> Self {
        let page: Self = glib::Object::builder().property("disk", disk).build();

        glib::spawn_future_local(glib::clone!(
            #[weak]
            page,
            async move {
                page.install(wipe).await;
            }
        ));

        page
    }

    pub async fn install(&self, wipe: bool) {
        let Some(disk) = self.disk() else {
            return;
        };

        let handle1 = disk.connect_install_phase_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                obj.imp().update_install_progress();
            }
        ));

        let handle2 = disk.connect_install_progress_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                obj.imp().update_install_progress();
            }
        ));

        if let Err(error) = disk.install(wipe).await {
            error!("Failed to install {error}");
        }

        disk.disconnect(handle1);
        disk.disconnect(handle2);

        // TODO: show error page if install failed
        self.go_next();
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/installer/install/install_page.blp")]
    #[properties(wrapper_type = super::InstallPage)]
    pub struct InstallPage {
        #[template_child]
        info: TemplateChild<adw::StatusPage>,

        #[template_child]
        progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        status: TemplateChild<gtk::Label>,

        #[property(get, set)]
        disk: RefCell<Option<Disk>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InstallPage {
        const NAME: &'static str = "SetupInstallPage";
        type Type = super::InstallPage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            osinfo::OsInfoCallbacks::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for InstallPage {
        fn constructed(&self) {
            self.parent_constructed();

            osinfo::bind_logo(self.info.get(), "icon-name");
        }
    }

    impl InstallPage {
        pub fn update_install_progress(&self) {
            let Some(disk) = self.obj().disk() else {
                return;
            };

            match disk.install_phase() {
                InstallPhase::None => {}
                InstallPhase::WarmUp => {
                    self.status.set_label(&gettext("Getting Warmed Up..."));
                    self.progress.set_fraction(0.0);
                }
                InstallPhase::Minimizing => {
                    self.status.set_label(&gettext("Minimizing"));
                    self.progress.set_fraction(0.0);
                }
                InstallPhase::Placing => {
                    self.status.set_label(&gettext("Placing"));
                    self.progress.set_fraction(0.0);
                }
                InstallPhase::WipingDisk => {
                    self.status.set_label(&gettext("Wiping disk"));
                    self.progress.set_fraction(0.0);
                }
                InstallPhase::WipingPartition => {
                    self.status.set_label(&gettext("Wiping Partition"));
                    self.progress.set_fraction(0.0);
                }
                InstallPhase::CopyingPartition => {
                    // Repart gives us progress only when copying partititon
                    let percent = disk.install_progress();
                    self.status.set_label(
                        &formatx!(gettext("Copying Partition ({percent}%)"), percent = percent)
                            .unwrap(),
                    );
                    self.progress.set_fraction(percent as f64 / 100.0);
                }
                InstallPhase::FormattingPartition => {
                    self.status.set_label(&gettext("Formatting Partition"));
                    self.progress.set_fraction(1.0);
                }
                InstallPhase::AdjustingPartition => {
                    self.status.set_label(&gettext("Adjusting Partition"));
                    self.progress.set_fraction(1.0);
                }
                InstallPhase::WritingTable => {
                    self.status.set_label(&gettext("Writing Partition Table"));
                    self.progress.set_fraction(1.0);
                }
                InstallPhase::Finished => {
                    self.status.set_label(&gettext("Finished"));
                    self.progress.set_fraction(1.0);
                }
            }
        }
    }

    impl WidgetImpl for InstallPage {}
    impl NavigationPageImpl for InstallPage {}
    impl BasePageImpl for InstallPage {}
}
