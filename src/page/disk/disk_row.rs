/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;

use std::cell::OnceCell;

use super::obj::{Disk, DiskStatus};

glib::wrapper! {
    pub struct DiskRow(ObjectSubclass<imp::DiskRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl DiskRow {
    pub fn new(disk: &Disk) -> Self {
        glib::Object::builder().property("disk", disk).build()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/disk/disk_row.blp")]
    #[properties(wrapper_type = super::DiskRow)]
    pub struct DiskRow {
        #[property(get, construct_only)]
        disk: OnceCell<Disk>,

        #[template_child]
        type_icon: TemplateChild<gtk::Image>,

        #[template_child]
        warning_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        info_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        details: TemplateChild<gtk::Popover>,
        #[template_child]
        details_title: TemplateChild<gtk::Label>,
        #[template_child]
        details_subtitle: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DiskRow {
        const NAME: &'static str = "SetupDiskRow";
        type Type = super::DiskRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DiskRow {
        fn constructed(&self) {
            self.parent_constructed();

            let disk = self.disk.get().unwrap();

            self.type_icon.set_icon_name(Some(&disk.icon_name()));
            self.obj().set_title(&disk.display_name());
            self.obj().set_subtitle(&disk.display_desc());

            self.warning_button.set_popover(Some(&self.details.get()));
            self.warning_button.set_visible(true);
            self.details_title.set_label("Asdf");
            self.details_subtitle.set_label("This is a description! Woohoo!");
        }
    }

    impl WidgetImpl for DiskRow {}
    impl ListBoxRowImpl for DiskRow {}
    impl PreferencesRowImpl for DiskRow {}
    impl ActionRowImpl for DiskRow {}
}
