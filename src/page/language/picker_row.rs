/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;

use std::cell::OnceCell;

use crate::models::language::Language;

glib::wrapper! {
    pub struct LanguagePickerRow(ObjectSubclass<imp::LanguagePickerRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl LanguagePickerRow {
    pub fn new(lang: &Language) -> Self {
        glib::Object::builder().property("language", lang).build()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/language/picker_row.blp")]
    #[properties(wrapper_type = super::LanguagePickerRow)]
    pub struct LanguagePickerRow {
        #[property(get, construct_only)]
        language: OnceCell<Language>,

        #[template_child]
        title: TemplateChild<gtk::Label>,
        #[template_child]
        subtitle: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LanguagePickerRow {
        const NAME: &'static str = "SetupLanguagePickerRow";
        type Type = super::LanguagePickerRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LanguagePickerRow {
        fn constructed(&self) {
            self.parent_constructed();

            let lang = self.language.get().unwrap();
            lang.bind_property("translated-display-name", &self.title.get(), "label")
                .sync_create()
                .build();
            lang.bind_property("display-name", &self.subtitle.get(), "label")
                .sync_create()
                .build();
        }
    }

    impl WidgetImpl for LanguagePickerRow {}
    impl ListBoxRowImpl for LanguagePickerRow {}
}
