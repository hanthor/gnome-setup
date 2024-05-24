/* SPDX-License-Identifier: GPL-3.0-or-later */

use gtk::{glib, gio};
use gio::prelude::*;
use gio::subclass::prelude::*;
use gnome_desktop as gutil;

use std::cmp::Ordering;

use super::{Language, LocaleParseError, POPULAR_LOCALES};

glib::wrapper! {
    pub struct LanguageModel(ObjectSubclass<imp::LanguageModel>)
        @implements gio::ListModel;
}

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct LanguageModel(gio::ListStore);

    #[glib::object_subclass]
    impl ObjectSubclass for LanguageModel {
        const NAME: &'static str = "SetupLanguageModel";
        type Type = super::LanguageModel;
        type Interfaces = (gio::ListModel,);

        fn new() -> Self {
            let model = gio::ListStore::new::<Language>();

            for id in gutil::all_locales() {
                let lang = match Language::new(&id) {
                    Ok(lang) => lang,
                    Err(LocaleParseError::NotUtf8) => continue,
                    Err(e) => {
                        log::warn!("Skipping {id}: {e}");
                        continue
                    }
                };

                model.append(&lang);
            }

            model.sort(|a: &glib::Object, b: &glib::Object| {
                let a: &Language = a.downcast_ref().unwrap();
                let b: &Language = b.downcast_ref().unwrap();

                // If we have a preselected language, sort it at the very top
                if let Some(preselect) = Language::system_default_id() {
                    match (a.id() == preselect, b.id() == preselect) {
                        (true, false) => return Ordering::Less,
                        (false, true) => return Ordering::Greater,
                        _ => (),
                    }
                }

                // Sort the popular languages at the top of the list
                let a_popular = POPULAR_LOCALES.iter().position(|&x| x == a.id());
                let b_popular = POPULAR_LOCALES.iter().position(|&x| x == b.id());
                match (a_popular, b_popular) {
                    (Some(_), None) => return Ordering::Less,
                    (None, Some(_)) => return Ordering::Greater,
                    (Some(a_idx), Some(b_idx)) => return a_idx.cmp(&b_idx),
                    _ => (),
                }

                a.sort_key().cmp(&b.sort_key())
            });

            Self(model)
        }
    }

    impl ObjectImpl for LanguageModel {}

    impl ListModelImpl for LanguageModel {
        fn item_type(&self) -> glib::Type {
            Language::static_type()
        }

        fn n_items(&self) -> u32 {
            self.0.n_items()
        }

        fn item(&self, pos: u32) -> Option<glib::Object> {
            self.0.item(pos)
        }
    }
}

impl Default for LanguageModel {
    fn default() -> Self {
        thread_local!(static ONCE: LanguageModel = glib::Object::new());
        ONCE.with(|v| v.clone())
    }
}

impl LanguageModel {
    pub fn iter(&self) -> impl Iterator<Item=Language> + '_ {
        self.upcast_ref::<gio::ListModel>()
            .iter::<Language>()
            .map(|l| l.unwrap())
    }
}
