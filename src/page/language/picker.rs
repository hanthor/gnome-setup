/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, gio};
use glib::subclass::Signal;

use gettextrs::gettext;
use std::cell::RefCell;
use std::sync::OnceLock;
use std::marker::PhantomData;

use crate::page::{self, BasePageImpl, BasePageExt};
use super::{Language, LanguageModel, LanguageFilter, LanguagePickerRow};

glib::wrapper! {
    pub struct LanguagePicker(ObjectSubclass<imp::LanguagePicker>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/language/picker.blp")]
    #[properties(wrapper_type = super::LanguagePicker)]
    pub struct LanguagePicker {
        filter: LanguageFilter,

        #[property(get)]
        model: gtk::FilterListModel,

        #[template_child]
        searchbox: TemplateChild<gtk::SearchEntry>,

        #[template_child]
        switcher: TemplateChild<gtk::Stack>,
        #[template_child]
        list: TemplateChild<gtk::ListBox>,
        #[template_child]
        empty: TemplateChild<adw::StatusPage>,

        #[property(get = Self::get_selected, set = Self::set_selected, nullable)]
        selected: PhantomData<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LanguagePicker {
        const NAME: &'static str = "SetupLanguagePicker";
        type Type = super::LanguagePicker;
        type ParentType = gtk::Box;

        fn new() -> Self {
            let filter = LanguageFilter::default();
            Self {
                filter: filter.clone(),
                model: gtk::FilterListModel::new(
                    Some(LanguageModel::default()),
                    Some(filter)
                ),
                .. Default::default()
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LanguagePicker {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![
                Signal::builder("selected")
                    .param_types([String::static_type()])
                    .build(),
            ])
        }

        fn constructed(&self) {
            self.parent_constructed();

            self.load_locale(); // Populate UI w/ initial language

            self.list.bind_model(Some(&self.model), |obj| {
                let lang = obj.downcast_ref::<Language>().unwrap();
                LanguagePickerRow::new(lang).upcast()
            });

            let obj = self.obj();
            self.model.connect_items_changed(
                glib::clone!(@strong obj => move |_, _, _, _| obj.imp().on_changed())
            );
        }
    }

    impl WidgetImpl for LanguagePicker {}
    impl BoxImpl for LanguagePicker {}

    #[gtk::template_callbacks]
    impl LanguagePicker {
        #[template_callback]
        fn on_search(&self) {
            self.searchbox.grab_focus();
            self.filter.set_query(&self.searchbox.text());
        }

        #[template_callback]
        fn on_row_selected(&self, row: Option<&gtk::ListBoxRow>) {
            let lang: Option<String> = row
                .map(|r| r.downcast_ref::<LanguagePickerRow>().unwrap())
                .map(|r| r.language().id());
            self.obj().emit_by_name::<()>("selected", &[&lang]);
        }

        fn on_changed(&self) {
            // We implement this manually, because ListBox's placeholder doesn't play well with
            // our scrolling AdwStatusPage
            // https://gitlab.gnome.org/GNOME/libadwaita/-/issues/636#note_2128279
            let child: gtk::Widget = if self.model.n_items() > 0 {
                self.list.get().upcast()
            } else {
                self.empty.get().upcast()
            };
            self.switcher.set_visible_child(&child);
        }

        pub(super) fn load_locale(&self) {
            self.searchbox.set_placeholder_text(Some(&gettext("Search languages")));
            self.empty.set_title(&gettext("No Results Found"));
            self.empty.set_description(Some(&gettext("Try a different search")));

            for lang in LanguageModel::default().iter() {
                lang.reload_locale();
            }
        }

        fn get_selected(&self) -> Option<String> {
            self.list
                .selected_row()
                .map(|r| r.downcast::<LanguagePickerRow>().unwrap())
                .map(|r| r.language().id())
        }

        fn set_selected(&self, selection: Option<String>) {
            let row: Option<LanguagePickerRow> = selection.and_then(|id| {
                (0i32..)
                    .zip(LanguageModel::default().iter())
                    .find(|(_, lang)| lang.id() == id)
                    .map(|(idx, _)| self.list.row_at_index(idx).unwrap().downcast().unwrap())
            });
            self.list.select_row(row.as_ref());
        }
    }
}

impl LanguagePicker {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn reload_locale(&self) {
        self.imp().load_locale()
    }
}
