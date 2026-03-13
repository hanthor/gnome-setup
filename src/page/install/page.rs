/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::base::{BasePage, BasePageExt, BasePageImpl};
use adw::prelude::*;
use adw::subclass::prelude::*;
use formatx::formatx;
use gettextrs::gettext;
use gtk::glib;
use std::cell::RefCell;

use super::done_page::DonePage;
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
    pub fn new(disk: &Disk) -> Self {
        glib::Object::builder().property("disk", disk).build()
    }

    pub async fn page(disk: Disk) -> BasePage {
        let page = Self::new(&disk);
        // FIXME: maybe start install once we showed the page?
        glib::spawn_future_local(glib::clone!(
            #[weak]
            page,
            async move {
                page.install().await;
            }
        ));
        page.upcast()
    }

    pub async fn install(&self) {
        let Some(disk) = self.disk() else {
            return;
        };

        self.imp().progress.set_fraction(0.0);
        self.imp().progress.set_pulse_step(0.5);

        disk.connect_install_phase_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                obj.update_install_progress();
            }
        ));

        disk.connect_install_progress_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                obj.update_install_progress();
            }
        ));

        disk.install(false).await;
        // TODO: show error page if install failed
        self.go_subpage(DonePage::page());
    }

    fn update_install_progress(&self) {
        let Some(disk) = self.disk() else {
            return;
        };

        match disk.install_phase() {
            InstallPhase::None => {}
            InstallPhase::WarmUp => {
                self.imp().status.set_label(&gettext("Getting Warmed Up..."));
                self.imp().progress.set_fraction(0.0);
            }
            InstallPhase::Minimizing => {
                self.imp().status.set_label(&gettext("Minimizing"));
                self.imp().progress.set_fraction(0.0);
            }
            InstallPhase::Placing => {
                self.imp().status.set_label(&gettext("Placing"));
                self.imp().progress.set_fraction(0.0);
            }
            InstallPhase::WipingDisk => {
                self.imp().status.set_label(&gettext("Wiping disk"));
                self.imp().progress.set_fraction(0.0);
            }
            InstallPhase::WipingPartition => {
                self.imp().status.set_label(&gettext("Wiping Partition"));
                self.imp().progress.set_fraction(0.0);
            }
            InstallPhase::CopyingPartition => {
                // Repart gives us progress only when copying partititon
                let percent = disk.install_progress();
                self.imp().status.set_label(
                    &formatx!(gettext("Copying Partition ({percent}%)"), percent = percent)
                        .unwrap(),
                );
                self.imp().progress.set_fraction(percent as f64 / 100.0);
            }
            InstallPhase::FormattingPartition => {
                self.imp().status.set_label(&gettext("Formatting Partition"));
                self.imp().progress.set_fraction(1.0);
            }
            InstallPhase::AdjustingPartition => {
                self.imp().status.set_label(&gettext("Adjusting Partition"));
                self.imp().progress.set_fraction(1.0);
            }
            InstallPhase::WritingTable => {
                self.imp().status.set_label(&gettext("Writing Partition Table"));
                self.imp().progress.set_fraction(1.0);
            }
            InstallPhase::Finished => {
                self.imp().status.set_label(&gettext("Finished"));
                self.imp().progress.set_fraction(1.0);
            }
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/install/page.blp")]
    #[properties(wrapper_type = super::InstallPage)]
    pub struct InstallPage {
        #[template_child]
        pub(super) info: TemplateChild<adw::StatusPage>,

        #[template_child]
        pub(super) progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) status: TemplateChild<gtk::Label>,

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

    impl WidgetImpl for InstallPage {}
    impl NavigationPageImpl for InstallPage {}
    impl BasePageImpl for InstallPage {}
}
