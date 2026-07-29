/* SPDX-License-Identifier: GPL-3.0-or-later */

use glib::prelude::*;
use glib::subclass::prelude::*;

use std::cell::{OnceCell, RefCell};

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for Keyboard {
        const NAME: &'static str = "SetupKeyboard";
        type Type = super::Keyboard;
    }

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Keyboard)]
    pub struct Keyboard {
        #[property(get, set, construct_only)]
        id: OnceCell<String>,

        #[property(get, set, construct_only)]
        pub(super) display_name: OnceCell<String>,

        #[property(get, set, construct_only)]
        pub(super) layout: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) variant: OnceCell<String>,

        #[property(get = Self::search_tokens, type = glib::StrV)]
        pub(super) search_tokens: RefCell<OnceCell<glib::StrV>>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for Keyboard {}

    impl Keyboard {
        fn search_tokens(&self) -> glib::StrV {
            let init = || {
                let src = format!(
                    "{} {} {}",
                    self.display_name.get().unwrap(),
                    self.layout.get().unwrap(),
                    &self.variant.get().unwrap().to_owned()
                );
                let (mut result, alts) = tokenize_and_casefold_with_alts(&src, None);
                result.extend(alts);
                result
            };
            self.search_tokens.borrow().get_or_init(init).clone()
        }
    }
}

glib::wrapper! {
    pub struct Keyboard(ObjectSubclass<imp::Keyboard>);
}

impl Keyboard {
    pub fn new(id: &str, display_name: &str, xkb_layout: &str, xkb_variant: &str) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("display_name", display_name)
            .property("layout", xkb_layout)
            .property("variant", xkb_variant)
            .build()
    }
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
