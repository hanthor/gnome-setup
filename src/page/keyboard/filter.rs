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

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::Cell;
    use std::rc::Rc;

    /// Runs `set_query` with each query in turn and returns the `FilterChange`
    /// the filter reported for the last one.
    fn last_change(queries: &[&str]) -> gtk::FilterChange {
        let filter = KeyboardFilter::default();
        let change = Rc::new(Cell::new(None));
        filter.connect_changed({
            let change = change.clone();
            move |_, c| change.set(Some(c))
        });
        for query in queries {
            filter.set_query(query);
        }
        change.get().expect("set_query should emit `changed`")
    }

    #[test]
    fn first_query_is_more_strict() {
        assert_eq!(last_change(&["fr"]), gtk::FilterChange::MoreStrict);
    }

    #[test]
    fn extending_the_query_is_more_strict() {
        assert_eq!(last_change(&["fr", "fre"]), gtk::FilterChange::MoreStrict);
    }

    #[test]
    fn shortening_the_query_is_less_strict() {
        assert_eq!(last_change(&["fre", "fr"]), gtk::FilterChange::LessStrict);
    }

    #[test]
    fn clearing_the_query_is_less_strict() {
        assert_eq!(last_change(&["fre", ""]), gtk::FilterChange::LessStrict);
    }

    #[test]
    fn unrelated_query_is_different() {
        assert_eq!(last_change(&["fre", "deu"]), gtk::FilterChange::Different);
    }

    #[test]
    fn empty_query_matches_everything() {
        let filter = KeyboardFilter::default();
        let keyboard = Keyboard::new("xkb:us::eng", "English (US)", "us", "");
        assert_eq!(filter.strictness(), gtk::FilterMatch::All);
        assert!(filter.match_(&keyboard));
    }

    #[test]
    fn every_query_token_must_prefix_a_keyboard_token() {
        let filter = KeyboardFilter::default();
        let keyboard = Keyboard::new("xkb:us:dvorak:eng", "English (Dvorak)", "us", "dvorak");

        filter.set_query("eng");
        assert_eq!(filter.strictness(), gtk::FilterMatch::Some);
        assert!(filter.match_(&keyboard));

        filter.set_query("ENG dvo");
        assert!(filter.match_(&keyboard));

        filter.set_query("eng colemak");
        assert!(!filter.match_(&keyboard));

        filter.set_query("xyz");
        assert!(!filter.match_(&keyboard));
    }
}
