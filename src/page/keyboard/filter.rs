/* SPDX-License-Identifier: GPL-3.0-or-later */

use gtk::prelude::*;
use gtk::subclass::prelude::*;

use std::cell::RefCell;

use crate::models::keyboard::Keyboard;
use crate::models::language::tokenize_and_casefold;

glib::wrapper! {
    pub struct KeyboardFilter(ObjectSubclass<imp::KeyboardFilter>)
        @extends gtk::Filter;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for KeyboardFilter {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl KeyboardFilter {
    pub fn set_query(&self, query: &str) {
        let prepared = tokenize_and_casefold(query);
        let change = {
            let old = &self.imp().query.borrow().0;
            #[allow(clippy::if_same_then_else)]
            if query.is_empty() {
                gtk::FilterChange::LessStrict
            } else if old.is_empty() {
                gtk::FilterChange::MoreStrict
            } else if query.starts_with(old) {
                gtk::FilterChange::MoreStrict
            } else if old.starts_with(query) {
                gtk::FilterChange::LessStrict
            } else {
                gtk::FilterChange::Different
            }
        };
        self.imp().query.replace((query.into(), prepared));
        self.changed(change);
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct KeyboardFilter {
        pub(super) query: RefCell<(String, glib::StrV)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyboardFilter {
        const NAME: &'static str = "SetupKeyboardFilter";
        type Type = super::KeyboardFilter;
        type ParentType = gtk::Filter;
    }

    impl ObjectImpl for KeyboardFilter {}

    impl FilterImpl for KeyboardFilter {
        fn strictness(&self) -> gtk::FilterMatch {
            if self.query.borrow().0.is_empty() {
                gtk::FilterMatch::All
            } else {
                gtk::FilterMatch::Some
            }
        }

        fn match_(&self, item: &glib::Object) -> bool {
            let query = &self.query.borrow().1;
            if query.is_empty() {
                return true;
            }

            let lang = item.downcast_ref::<Keyboard>().unwrap();
            let lang_tokens = lang.search_tokens();

            // Like g_str_match_string, we only match if each query token is the prefix
            // of one of the keyboard's search tokens
            'outer: for query_tok in query.iter() {
                for lang_tok in lang_tokens.iter() {
                    if lang_tok.starts_with(query_tok.as_str()) {
                        continue 'outer;
                    }
                }
                return false;
            }
            true
        }
    }
}
