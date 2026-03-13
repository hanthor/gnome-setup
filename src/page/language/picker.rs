/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::models::language::{Language, LanguageModel};

use super::filter::LanguageFilter;
use super::picker_row::LanguagePickerRow;

glib::wrapper! {
    pub struct LanguagePicker(ObjectSubclass<imp::LanguagePicker>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

mod imp {
    use super::*;

    use glib::clone;
    use std::cell::RefCell;

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

        #[property(get = Self::selected, set = Self::set_selected, nullable, explicit_notify)]
        selected: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LanguagePicker {
        const NAME: &'static str = "SetupLanguagePicker";
        type Type = super::LanguagePicker;
        type ParentType = gtk::Box;

        fn new() -> Self {
            let filter = LanguageFilter::default();
            let model =
                gtk::FilterListModel::new(Some(LanguageModel::default()), Some(filter.clone()));
            Self {
                filter,
                model,
                ..Default::default()
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
        fn constructed(&self) {
            self.parent_constructed();

            self.load_locale(); // Populate UI w/ initial language

            self.list.bind_model(
                Some(&self.model),
                clone!(
                    #[weak(rename_to = this)]
                    self,
                    #[upgrade_or_panic]
                    move |obj| {
                        let lang = obj.downcast_ref::<Language>().unwrap();

                        let row = LanguagePickerRow::new(lang);
                        if this.selected.borrow().as_ref() == Some(&lang.id()) {
                            this.select_row(&row);
                        }

                        row.upcast()
                    }
                ),
            );

            let obj = self.obj();
            self.model.connect_items_changed(glib::clone!(
                #[strong]
                obj,
                move |_, _, _, _| obj.imp().on_changed()
            ));
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
        fn on_row_activated(&self, row: &LanguagePickerRow) {
            let selected = row.language().id();

            self.selected.replace(Some(selected));
            self.select_row(row);
            self.obj().notify("selected");
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

        fn selected(&self) -> Option<String> {
            self.selected.borrow().to_owned()
        }

        fn set_selected(&self, selection: Option<String>) {
            let row: Option<LanguagePickerRow> = if let Some(ref selection) = selection {
                self.model
                    .iter::<glib::Object>()
                    .enumerate()
                    .find(|(_, lang)| {
                        &lang.as_ref().unwrap().downcast_ref::<Language>().unwrap().id()
                            == selection
                    })
                    .map(|(idx, _)| self.list.row_at_index(idx as i32).unwrap().downcast().unwrap())
            } else {
                None
            };

            self.selected.replace(selection);
            if let Some(row) = row {
                self.select_row(&row);
            }
            self.obj().notify("selected");
        }

        fn select_row(&self, row: &LanguagePickerRow) {
            row.set_state_flags(gtk::StateFlags::SELECTED, false);
            row.update_state(&[gtk::accessible::State::Selected(Some(true))]);
        }
    }
}
