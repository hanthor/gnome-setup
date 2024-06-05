use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use std::cell::RefCell;

use super::obj::Language;

glib::wrapper! {
    pub struct LanguageFilter(ObjectSubclass<imp::LanguageFilter>)
        @extends gtk::Filter;
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct LanguageFilter {
        pub(super) query: RefCell<(String, glib::StrV)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LanguageFilter {
        const NAME: &'static str = "SetupLanguageFilter";
        type Type = super::LanguageFilter;
        type ParentType = gtk::Filter;
    }

    impl ObjectImpl for LanguageFilter {}

    impl FilterImpl for LanguageFilter {
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

            let lang = item.downcast_ref::<Language>().unwrap();
            let lang_tokens = lang.search_tokens();

            // Like g_str_match_string, we only match if each query token is the prefix
            // of one of the language's search tokens
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

impl Default for LanguageFilter {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl LanguageFilter {
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
            } else  {
                gtk::FilterChange::Different
            }
        };
        self.imp().query.replace((query.into(), prepared));
        self.changed(change);
    }
}

fn tokenize_and_casefold_internal(
    arg: &str,
    translit_locale: Option<&str>,
    alts: bool
) -> (glib::StrV, Option<glib::StrV>) {
    // https://github.com/gtk-rs/gtk-rs-core/issues/1418
    use glib::translate::*;
    arg.run_with_gstr(|arg| {
        translit_locale.run_with_gstr(|translit_locale| {
            let mut alternates: *mut *mut std::ffi::c_char = std::ptr::null_mut();
            let alternates_ptr = if alts {
                &mut alternates
            } else {
                std::ptr::null_mut()
            };
            unsafe {
                let result = glib::ffi::g_str_tokenize_and_fold(
                    arg.to_glib_none().0,
                    translit_locale.to_glib_none().0,
                    alternates_ptr
                );
                (
                    glib::StrV::from_glib_full(result),
                    alts.then(|| glib::StrV::from_glib_full(alternates))
                )
            }
        })
    })
}

pub fn tokenize_and_casefold(arg: &str) -> glib::StrV {
    tokenize_and_casefold_internal(arg, None, false).0
}

pub fn tokenize_and_casefold_with_alts(
    arg: &str,
    translit_locale: Option<&str>
) -> (glib::StrV, glib::StrV) {
    let res = tokenize_and_casefold_internal(arg, translit_locale, true);
    (res.0, res.1.unwrap())
}
