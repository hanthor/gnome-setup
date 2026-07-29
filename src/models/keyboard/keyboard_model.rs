/* SPDX-License-Identifier: GPL-3.0-or-later */

use gio::prelude::*;
use gio::subclass::prelude::*;

use crate::interfaces::LocaledProxy;
use glib::clone;
use gnome_desktop as gutil;
use gtk::gio;
use gutil::{XkbInfo, XkbInfoExt};
use indexmap::IndexMap;
use std::cell::RefCell;

use super::keyboard::Keyboard;

glib::wrapper! {
    pub struct KeyboardModel(ObjectSubclass<imp::KeyboardModel>)
        @implements gio::ListModel;
}

impl Default for KeyboardModel {
    fn default() -> Self {
        thread_local!(static ONCE: KeyboardModel = glib::Object::new());
        ONCE.with(|v| v.clone())
    }
}

impl KeyboardModel {
    pub fn iter(&self) -> impl Iterator<Item = Keyboard> + '_ {
        self.upcast_ref::<gio::ListModel>().iter::<Keyboard>().map(|l| l.unwrap())
    }

    pub fn reload_keyboards(&self) {
        glib::spawn_future_local(clone!(
            #[weak(rename_to = this)]
            self,
            async move {
                let removed = {
                    let mut list = this.imp().list.borrow_mut();
                    let removed = list.len();

                    *list = IndexMap::new();
                    removed
                };

                this.imp().load_keyboards().await;

                let added = this.imp().list.borrow().len();
                this.items_changed(0, removed as u32, added as u32);
            }
        ));
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct KeyboardModel {
        pub(super) list: RefCell<IndexMap<String, Keyboard>>,
        info: XkbInfo,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyboardModel {
        const NAME: &'static str = "SetupKeyboardModel";
        type Type = super::KeyboardModel;
        type Interfaces = (gio::ListModel,);
    }

    impl KeyboardModel {
        fn extend_keyboards<I: IntoIterator<Item = impl AsRef<str>>>(&self, list: I) {
            let iter = list.into_iter();
            let mut map = self.list.borrow_mut();

            for id in iter {
                let id = id.as_ref();

                if map.contains_key(id) {
                    continue;
                }

                let Some((Some(display_name), _short_name, Some(xkb_layout), Some(xkb_variant))) =
                    self.info.layout_info(id)
                else {
                    log::warn!("Could not query layout info for {id}");
                    continue;
                };

                let keyboard = Keyboard::new(
                    id,
                    display_name.as_str(),
                    xkb_layout.as_str(),
                    xkb_variant.as_str(),
                );

                map.insert(id.to_owned(), keyboard);
            }
        }
    }

    impl ObjectImpl for KeyboardModel {
        fn constructed(&self) {
            self.parent_constructed();

            glib::spawn_future_local(clone!(
                #[weak(rename_to = this)]
                self,
                async move {
                    this.load_keyboards().await;
                    let added = this.list.borrow().len();
                    this.obj().items_changed(0, 0, added as u32);
                }
            ));
        }
    }

    impl KeyboardModel {
        async fn add_current_keyboards(&self) {
            let localed = match LocaledProxy::new(crate::system_bus()).await {
                Ok(proxy) => proxy,
                Err(e) => {
                    log::error!("Failed to obtain localed proxy: {e}");
                    return;
                }
            };

            let keyborads = localed.current_keyboards().await;
            self.extend_keyboards(keyborads);
        }

        pub(super) async fn load_keyboards(&self) {
            let info = XkbInfo::new();
            let locale = crate::locale();

            self.add_current_keyboards().await;

            if let Some((_type, id)) = gutil::input_source_from_locale(locale.as_str()) {
                self.extend_keyboards([id]);
            }

            if let Some((lang, country, _, _)) = gutil::parse_locale(locale.as_str()) {
                let mut list = info.layouts_for_language(lang.as_str());
                list.sort();
                self.extend_keyboards(list);

                if let Some(country) = country {
                    let mut list = info.layouts_for_country(country.as_str());
                    list.sort();
                    self.extend_keyboards(list);
                }
            }

            let mut list = info.all_layouts();
            list.sort();
            self.extend_keyboards(list);
        }
    }

    impl ListModelImpl for KeyboardModel {
        fn item_type(&self) -> glib::Type {
            Keyboard::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, pos: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(pos as usize)
                .map(|(_id, keyboard)| keyboard.clone().upcast())
        }
    }
}
