/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::models::keyboard::{Keyboard, KeyboardModel};

use super::filter::KeyboardFilter;
use super::picker_row::KeyboardPickerRow;

glib::wrapper! {
    pub struct KeyboardPicker(ObjectSubclass<imp::KeyboardPicker>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl KeyboardPicker {
    pub fn set_selected_by_id(&self, selection: Option<&str>) {
        let row: Option<KeyboardPickerRow> = if let Some(ref selection) = selection {
            self.imp()
                .model
                .iter::<glib::Object>()
                .enumerate()
                .find(|(_, keyboard)| {
                    &keyboard.as_ref().unwrap().downcast_ref::<Keyboard>().unwrap().id()
                        == selection
                })
                .map(|(idx, _)| {
                    self.imp().list.row_at_index(idx as i32).unwrap().downcast().unwrap()
                })
        } else {
            None
        };

        self.imp().select_row(row.as_ref());
    }
}

mod imp {
    use super::*;

    
    use glib::collections::strv::StrV;
    use std::cell::RefCell;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/keyboard/picker.blp")]
    #[properties(wrapper_type = super::KeyboardPicker)]
    pub struct KeyboardPicker {
        filter: KeyboardFilter,

        #[property(get)]
        pub(super) model: gtk::FilterListModel,

        #[template_child]
        searchbox: TemplateChild<gtk::SearchEntry>,

        #[template_child]
        switcher: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) list: TemplateChild<gtk::ListBox>,
        #[template_child]
        empty: TemplateChild<adw::StatusPage>,

        #[property(get = Self::selected, nullable, explicit_notify, type = Option<Keyboard>)]
        pub(super) selected: glib::WeakRef<KeyboardPickerRow>,
        #[property(get = Self::keyboards, set = Self::set_keyboards)]
        keyboards: RefCell<StrV>,
        keyboards_filter: gtk::CustomFilter,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyboardPicker {
        const NAME: &'static str = "SetupKeyboardPicker";
        type Type = super::KeyboardPicker;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for KeyboardPicker {
        fn constructed(&self) {
            self.parent_constructed();

            let keyboard_model = KeyboardModel::default();

            let filtered = gtk::FilterListModel::builder()
                .model(&keyboard_model)
                .filter(&self.keyboards_filter)
                .build();

            keyboard_model.connect_items_changed(glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _, _, _| {
                    if this.selected.upgrade().is_none() {
                        if let Some(row) = this.list.row_at_index(0) {
                            this.select_row(Some(row.downcast_ref().unwrap()));
                        }
                    } else {
                        println!("Is some");
                    }
                }
            ));

            self.model.set_filter(Some(&self.filter));
            self.model.set_model(Some(&filtered));

            self.list.bind_model(Some(&self.model), move |obj| {
                let keyboard = obj.downcast_ref::<Keyboard>().unwrap();
                KeyboardPickerRow::new(keyboard).upcast()
            });

            let obj = self.obj();
            self.model.connect_items_changed(glib::clone!(
                #[strong]
                obj,
                move |_, _, _, _| obj.imp().on_changed()
            ));

            if self.selected.upgrade().is_none() {
                if let Some(row) = self.list.row_at_index(0) {
                    self.select_row(Some(row.downcast_ref().unwrap()));
                }
            }
        }
    }

    impl WidgetImpl for KeyboardPicker {}
    impl BoxImpl for KeyboardPicker {}

    #[gtk::template_callbacks]
    impl KeyboardPicker {
        fn keyboards(&self) -> StrV {
            self.keyboards.borrow().clone()
        }

        fn set_keyboards(&self, keyboards: StrV) {
            self.keyboards.replace(keyboards.clone());

            if keyboards.is_empty() {
                self.keyboards_filter.unset_filter_func();
            } else {
                self.keyboards_filter.set_filter_func(move |item| {
                    let keyboard = item.downcast_ref::<Keyboard>().unwrap();
                    keyboards.contains(keyboard.id())
                });
            }
        }

        #[template_callback]
        fn on_search(&self) {
            self.searchbox.grab_focus();
            self.filter.set_query(&self.searchbox.text());
        }

        #[template_callback]
        fn on_row_activated(&self, row: &KeyboardPickerRow) {
            self.select_row(Some(row));
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

        fn selected(&self) -> Option<Keyboard> {
            let selected = self.selected.upgrade();
            Some(selected?.keyboard())
        }

        pub(super) fn select_row(&self, row: Option<&KeyboardPickerRow>) {
            let selected = self.selected.upgrade();
            if let Some(old_row) = selected {
                old_row.unset_state_flags(gtk::StateFlags::SELECTED);
                old_row.update_state(&[gtk::accessible::State::Selected(Some(false))]);
            }

            if let Some(row) = row {
                row.set_state_flags(gtk::StateFlags::SELECTED, false);
                row.update_state(&[gtk::accessible::State::Selected(Some(true))]);
            }

            self.selected.set(row);
            self.obj().notify("selected");
        }
    }
}
