/* SPDX-License-Identifier: GPL-3.0-or-later */

use glib::prelude::*;
use glib::subclass::prelude::*;

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::OnceLock;

use gnome_desktop as gutil;
use thiserror::Error;

glib::wrapper! {
    pub struct Language(ObjectSubclass<imp::Language>);
}

impl Language {
    pub fn new(locale: &str) -> Result<Self, LocaleParseError> {
        let parsed = ParsedLocale::new(locale)?;

        let obj: Self = glib::Object::new();
        obj.imp().parsed.set(parsed).unwrap();

        Ok(obj)
    }

    pub fn reload_locale(&self) {
        self.imp().display_name.borrow_mut().take();
        self.imp().search_tokens.borrow_mut().take();

        self.notify_display_name();
        self.notify_search_tokens();
    }

    pub fn system_default_id() -> Option<&'static str> {
        // If the system already has a language set (from a previous invocation of Setup on a
        // previous boot, or set by a VM hypervisor, or compiled-in defaults, or from wherever else),
        // systemd will set LANG to that value for us. If no such default exists, systemd will set
        // the language to C.UTF-8.
        static ONCE: OnceLock<Option<String>> = OnceLock::new();
        ONCE.get_or_init(|| std::env::var("LANG").ok().filter(|l| l != "C.UTF-8"))
            .as_deref()
    }
}

#[derive(Error, Debug)]
pub enum LocaleParseError {
    #[error("invalid locale")]
    Invalid,

    #[error("non-utf8 locale")]
    NotUtf8,
}

#[derive(Debug)]
struct ParsedLocale {
    id: String,
    lang: String,
    country: Option<String>,
    modifier: Option<String>,
}

impl ParsedLocale {
    fn new(locale: &str) -> Result<Self, LocaleParseError> {
        use LocaleParseError as E;

        let (lang, country, encoding, modifier) = gutil::parse_locale(locale).ok_or(E::Invalid)?;

        if encoding.as_deref() != Some("UTF-8") {
            return Err(E::NotUtf8);
        }

        Ok(ParsedLocale {
            id: locale.to_string(),
            lang: lang.into(),
            country: country.map(Into::into),
            modifier: modifier.map(Into::into),
        })
    }

    fn display_name(&self, translate_to: Option<&str>) -> String {
        let name = translate_to
            .is_some()
            .then(|| gutil::language_from_code(&self.lang, translate_to))
            .flatten()
            .or_else(|| gutil::language_from_code(&self.lang, None))
            .map(Into::<String>::into)
            .unwrap();

        let country = self.country.as_ref().and_then(|country| {
            translate_to
                .is_some()
                .then(|| gutil::country_from_code(country, translate_to))
                .flatten()
                .or_else(|| gutil::country_from_code(country, None))
            //.map(Into::<String>::intocountry_from_code)
        });
        let country = country.as_deref();

        let modifier = self.modifier.as_ref().and_then(|modifier| {
            translate_to
                .is_some()
                .then(|| gutil::translated_modifier(modifier, translate_to))
                .flatten()
                .or_else(|| gutil::translated_modifier(modifier, None))
                .map(Into::<String>::into)
        });
        let modifier = modifier.as_deref();

        let mut disp = String::new();
        disp.push_str(&name);

        if let Some(modifier) = modifier {
            disp.push_str(" (");
            disp.push_str(modifier);
            disp.push(')');
        }

        match country {
            Some(country) if should_show_country(&self.lang, modifier) => {
                disp.push_str(" — ");
                disp.push_str(country);
            }
            _ => (),
        }

        disp.shrink_to_fit();
        disp
    }
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for Language {
        const NAME: &'static str = "SetupLanguage";
        type Type = super::Language;
    }

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Language)]
    pub struct Language {
        pub(super) parsed: OnceCell<ParsedLocale>,

        #[property(get = Self::id)]
        id: PhantomData<String>,

        #[property(get = Self::display_name)]
        pub(super) display_name: RefCell<OnceCell<String>>,
        #[property(get = Self::translated_display_name)]
        translated_display_name: OnceCell<String>,
        #[property(get = Self::english_display_name)]
        english_display_name: OnceCell<String>,

        #[property(get = Self::search_tokens, type = glib::StrV)]
        pub(super) search_tokens: RefCell<OnceCell<glib::StrV>>,

        #[property(get = Self::sort_key)]
        sort_key: OnceCell<String>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for Language {}

    impl Language {
        fn id(&self) -> String {
            self.parsed.get().unwrap().id.clone()
        }

        fn display_name(&self) -> String {
            let init = || {
                let parsed = self.parsed.get().unwrap();
                parsed.display_name(None)
            };
            self.display_name.borrow().get_or_init(init).clone()
        }

        fn translated_display_name(&self) -> String {
            let init = || {
                let parsed = self.parsed.get().unwrap();
                parsed.display_name(Some(parsed.id.as_str()))
            };
            self.translated_display_name.get_or_init(init).clone()
        }

        fn english_display_name(&self) -> String {
            let init = || {
                let parsed = self.parsed.get().unwrap();
                parsed.display_name(Some("en_US.UTF-8"))
            };
            self.english_display_name.get_or_init(init).clone()
        }

        fn search_tokens(&self) -> glib::StrV {
            let init = || {
                let src = self.display_name()
                    + " "
                    + &self.translated_display_name()
                    + " "
                    + &self.english_display_name();
                let translit = Some(self.parsed.get().unwrap().id.as_str());
                let (mut result, alts) = tokenize_and_casefold_with_alts(&src, translit);
                result.extend(alts);
                result
            };
            self.search_tokens.borrow().get_or_init(init).clone()
        }

        fn sort_key(&self) -> String {
            // Adapted from cc_util_normalize_casefold_and_unaccent, under the GPL
            // https://gitlab.gnome.org/GNOME/gnome-control-center/-/blob/2cf5b90c/panels/common/cc-util.c
            let init = || {
                let mut key: String = glib::casefold(glib::normalize(
                    self.translated_display_name(),
                    glib::NormalizeMode::All, // un-combine accented characters
                ))
                .into();

                // Remove combining diacritical marks (which were previously extracted above,
                // in the normalization step). This has the effect if un-accenting the string
                key.retain(|c| match c {
                    '\u{0300}'..='\u{036F}' => false, // Basic range
                    '\u{1DC0}'..='\u{1DFF}' => false, // Supplement
                    '\u{20D0}'..='\u{20FF}' => false, // For symbols
                    '\u{FE20}'..='\u{FE2F}' => false, // Half marks
                    _ => true,
                });

                key
            };
            self.sort_key.get_or_init(init).clone()
        }
    }
}

fn should_show_country(lang: &str, modifier: Option<&str>) -> bool {
    // Cache mapping (lang, modifier) => number_of_countries
    static COUNTS: OnceLock<HashMap<(String, Option<String>), u8>> = OnceLock::new();
    let counts = COUNTS.get_or_init(|| {
        let mut counts: HashMap<(String, Option<String>), u8> = HashMap::new();
        for id in gutil::all_locales() {
            let Ok(parsed) = ParsedLocale::new(&id) else {
                continue;
            };
            let entry = counts.entry((parsed.lang, parsed.modifier)).or_default();
            *entry += 1;
        }
        counts.shrink_to_fit();
        counts
    });

    let n_countries = counts.get(&(lang.into(), modifier.map(Into::into))).copied().unwrap_or(0);

    // If a given language + modifier combo has more than one country associated with it,
    // then we show the country in the UI. Otherwise we don't, because there's only one obvious
    // country that we know uses that variant of that language
    n_countries > 1
}

fn tokenize_and_casefold_internal(
    arg: &str,
    translit_locale: Option<&str>,
    alts: bool,
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
                    alternates_ptr,
                );
                (
                    glib::StrV::from_glib_full(result),
                    alts.then(|| glib::StrV::from_glib_full(alternates)),
                )
            }
        })
    })
}

fn tokenize_and_casefold_with_alts(
    arg: &str,
    translit_locale: Option<&str>,
) -> (glib::StrV, glib::StrV) {
    let res = tokenize_and_casefold_internal(arg, translit_locale, true);
    (res.0, res.1.unwrap())
}

pub fn tokenize_and_casefold(arg: &str) -> glib::StrV {
    tokenize_and_casefold_internal(arg, None, false).0
}
