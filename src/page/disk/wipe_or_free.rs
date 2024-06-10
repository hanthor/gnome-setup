/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use gettextrs::gettext;

use super::confirm;
use crate::page::InstallPage;

glib::wrapper! {
    pub struct WipeOrFreePage(ObjectSubclass<imp::WipeOrFreePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for WipeOrFreePage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl WipeOrFreePage {
    pub async fn page() -> BasePage {
        Self::default().upcast()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/disk/wipe_or_free.blp")]
    pub struct WipeOrFreePage {
        #[template_child]
        heading: TemplateChild<adw::StatusPage>,
        #[template_child]
        disk_info: TemplateChild<adw::ActionRow>,
        #[template_child]
        free_radio: TemplateChild<gtk::CheckButton>,
        #[template_child]
        go_btn: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WipeOrFreePage {
        const NAME: &'static str = "SetupWipeOrFreePage";
        type Type = super::WipeOrFreePage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WipeOrFreePage {
        fn constructed(&self) {
            self.parent_constructed();

            let disk_name = "Sandmug 960 EVO";
            let disk_summary = "128 GB Used — 768 GB Free";
            let disk_icon = "drive-harddisk-solidstate-symbolic";

            self.heading.set_description(Some(&gettext!(
                "Data detected on <b>{}</b>. Setup can use the full disk for installation, which will erase all software, documents, and data, or install into the free space, preserving existing data.",
                disk_name
            )));

            self.disk_info.set_title(disk_name);
            self.disk_info.set_subtitle(disk_summary);
            self.disk_info.set_icon_name(Some(disk_icon));

            self.selection_changed();
        }
    }

    impl WidgetImpl for WipeOrFreePage {}
    impl NavigationPageImpl for WipeOrFreePage {}
    impl BasePageImpl for WipeOrFreePage {}

    #[gtk::template_callbacks]
    impl WipeOrFreePage {
        #[template_callback]
        fn selection_changed(&self) {
            if self.free_radio.is_active() {
                // TRANSLATORS: This button immediately begins installation of the OS
                self.go_btn.set_label(&gettext("_Install"));
                self.go_btn.add_css_class("suggested-action");
            } else {
                // TRANSLATORS: This button prompts for confirmation before beginning installation
                self.go_btn.set_label(&gettext("_Install..."));
                self.go_btn.remove_css_class("suggested-action");
            }
        }

        #[template_callback]
        async fn install(&self) {
            if !self.free_radio.is_active() {
                if !confirm::expected_wipe(&*self.obj()).await {
                    return;
                }
            }

            self.obj().go_next(InstallPage::page());
        }
    }
}
