/* SPDX-License-Identifier: GPL-3.0-or-later */

use std::cell::{OnceCell, RefCell};

use adw::prelude::*;
use adw::subclass::prelude::*;

use glib::collections::ptr_slice::PtrSlice;
use gtk::gio;
use gtk::glib::clone;
use log::{error, warn};

use crate::models::keyboard::KeyboardModel;
use crate::models::language::{Language, LanguageModel};
use crate::page::{
    BasePage, BasePageExt, DonePage, EulaPage, InstallerPage, KeyboardPage, LanguagePage,
    PrereleasePage, RecoveryKeyPage, WelcomePage,
};

use crate::config::PAGES_CONFIG_DIR;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gtk::Native, gtk::Root, gio::ActionGroup, gio::ActionMap, gtk::ConstraintTarget, gtk::Accessible, gtk::Buildable, gtk::ShortcutManager;
}

impl Window {
    pub fn new(application: &impl IsA<gtk::Application>) -> Self {
        glib::Object::builder().property("application", application).build()
    }

    pub fn locale_changed(&self) {
        for lang in LanguageModel::default().iter() {
            lang.reload_locale();
        }

        KeyboardModel::default().reload_keyboards();

        let stack: Vec<adw::NavigationPage> = self
            .imp()
            .nav
            .navigation_stack()
            .iter::<adw::NavigationPage>()
            .map(|page| {
                page.ok()
                    .and_downcast::<BasePage>()
                    .unwrap()
                    .rebuild()
                    .upcast::<adw::NavigationPage>()
            })
            .collect();

        if let Some(first_page) = stack.first() {
            first_page.set_can_pop(false);
        }

        self.imp().nav.replace(&stack);
    }

    pub fn next_page(&self) {
        let imp = self.imp();
        let mut pages = imp.pages.borrow_mut();
        let mut pushed_pages = imp.pushed_pages.borrow_mut();

        let next_page = loop {
            if let Some(page_name) = pages.get(pushed_pages.len()) {
                if let Some(page) = self.imp().build_page_for_name(page_name) {
                    break page;
                } else {
                    // Remove the page we can't build
                    pages.remove(pushed_pages.len());
                }
            } else {
                self.close();
                return;
            }
        };

        imp.nav.push(&next_page);
        pushed_pages.push(next_page);
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/window.blp")]
    pub struct Window {
        #[template_child]
        pub nav: TemplateChild<adw::NavigationView>,
        pub pages: RefCell<Vec<glib::GStringPtr>>,
        pub pushed_pages: RefCell<Vec<BasePage>>,
        pub config: OnceCell<glib::KeyFile>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "SetupWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl Window {
        pub(super) fn load_pages_config(&self) {
            let key_file = glib::KeyFile::new();
            if let Err(error) = key_file.load_from_dirs(
                "pages.conf",
                &["/etc/gnome-setup/", PAGES_CONFIG_DIR],
                glib::KeyFileFlags::NONE,
            ) {
                log::error!("Failed to read pages config: {error}");
            }

            crate::interfaces::set_post_install_unit(
                key_file.string("page:installer", "post-install-unit").ok().map(Into::into),
            );

            // TODO: get from env
            let mode = "live-initial-setup";

            if let Ok(config_pages) = key_file.string_list("modes", mode) {
                let config_pages = config_pages.into_iter().flat_map(|name| {
                    if name.starts_with("%") {
                        match key_file.string_list("modes", &name) {
                            Ok(list) => list,
                            Err(error) => {
                                error!("Failed to parse variable `{name}`: {error}");
                                PtrSlice::new()
                            }
                        }
                    } else {
                        PtrSlice::from([name])
                    }
                });

                self.pages.replace(config_pages.collect());
            }
            self.config.set(key_file).unwrap();
        }

        pub(super) fn build_page_for_name(&self, page_name: &glib::GStringPtr) -> Option<BasePage> {
            let config = self.config.get().unwrap();
            let page = match page_name.as_str() {
                "installer-welcome" => WelcomePage::new().upcast(),
                "installer" => InstallerPage::new().upcast(),
                "recovery-key" => RecoveryKeyPage::new().upcast(),
                "installer-done" => DonePage::new().upcast(),
                "language" => {
                    let page = LanguagePage::new();
                    let config_group = format!("page:{}", page_name);

                    if config.has_group(&config_group) {
                        if let Ok(languages) = config.string_list(&config_group, "languages") {
                            let len = languages.len();
                            // FIXME: There is no conversation: https://github.com/gtk-rs/gtk-rs-core/pull/1915
                            use glib::translate::IntoGlibPtr;
                            let languages: glib::StrV = unsafe {
                                glib::StrV::from_glib_full_num(languages.into_glib_ptr(), len, true)
                            };
                            page.set_languages(languages);
                        }
                        page.set_selected_language(Language::system_default_id());
                    }
                    page.upcast()
                }
                "keyboard" => {
                    let page = KeyboardPage::new();
                    let config_group = format!("page:{}", page_name);

                    if config.has_group(&config_group) {
                        if let Ok(layouts) = config.string_list(&config_group, "layouts") {
                            let len = layouts.len();
                            // FIXME: There is no conversation: https://github.com/gtk-rs/gtk-rs-core/pull/1915
                            use glib::translate::IntoGlibPtr;
                            let layouts: glib::StrV = unsafe {
                                glib::StrV::from_glib_full_num(layouts.into_glib_ptr(), len, true)
                            };
                            page.set_keyboards(layouts);
                        }
                    }
                    page.upcast()
                }
                "prerelease-warning" => PrereleasePage::new()?.upcast(),
                "eula" => EulaPage::new()?.upcast(),
                "try-or-install" => WelcomePage::new().upcast(),
                page_name => {
                    warn!("Pages config contains unsupported page `{page_name}`");
                    return None;
                }
            };

            Some(page)
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();

            self.load_pages_config();

            self.nav.connect_popped(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, page| {
                    let mut pushed_pages = this.pushed_pages.borrow_mut();

                    if let Some(last_page) = pushed_pages.last() {
                        if last_page == page {
                            pushed_pages.pop();
                        }
                    }
                }
            ));

            self.obj().next_page();
            if let Some(page) = self.nav.visible_page() {
                page.set_can_pop(false);
            }
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}
