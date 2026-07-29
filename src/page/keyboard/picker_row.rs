/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;

use std::cell::OnceCell;

use crate::models::keyboard::Keyboard;

glib::wrapper! {
    pub struct KeyboardPickerRow(ObjectSubclass<imp::KeyboardPickerRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl KeyboardPickerRow {
    pub fn new(lang: &Keyboard) -> Self {
        glib::Object::builder().property("keyboard", lang).build()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/keyboard/picker_row.blp")]
    #[properties(wrapper_type = super::KeyboardPickerRow)]
    pub struct KeyboardPickerRow {
        #[property(get, construct_only, type = Keyboard)]
        keyboard: OnceCell<Keyboard>,

        #[template_child]
        title: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyboardPickerRow {
        const NAME: &'static str = "SetupKeyboardPickerRow";
        type Type = super::KeyboardPickerRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for KeyboardPickerRow {
        fn constructed(&self) {
            self.parent_constructed();

            let keyboard = self.keyboard.get().unwrap();
            keyboard
                .bind_property("display-name", &self.title.get(), "label")
                .sync_create()
                .build();
        }
    }

    impl WidgetImpl for KeyboardPickerRow {}
    impl ListBoxRowImpl for KeyboardPickerRow {}
}
