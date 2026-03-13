/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::base::{BasePage, BasePageExt, BasePageImpl, BasePageImplExt};
use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::collections::strv::StrV;

use std::cell::{OnceCell, RefCell};
use std::time::Duration;

use gettextrs::gettext;

use crate::interfaces::LocaledProxy;
use crate::models::language::POPULAR_LOCALES;
use crate::window::Window;

use super::picker::LanguagePicker;
use crate::page::PrereleasePage as Next;

/*
use zbus::{proxy};
#[proxy(
    interface = "org.freedesktop.locale1",
    default_service = "org.freedesktop.locale1",
    default_path = "/org/freedesktop/locale1"
)]
pub trait Localed {
    #[zbus(property(emits_changed_signal = "true"))]
    fn locale(&self) -> zbus::Result<Vec<String>>;
    fn set_locale(&self, locale: Vec<String>, interactive: bool) -> zbus::Result<()>;
}
*/

glib::wrapper! {
    pub struct LanguagePage(ObjectSubclass<imp::LanguagePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LanguagePage {
    pub fn page() -> BasePage {
        glib::Object::new::<LanguagePage>().upcast()
    }
}

mod imp {
    use super::*;
    use std::marker::PhantomData;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/language/page.blp")]
    #[properties(wrapper_type = super::LanguagePage)]
    pub struct LanguagePage {
        #[template_child]
        welcome: TemplateChild<adw::Carousel>,
        welcome_anim: RefCell<Option<glib::SourceId>>,

        #[template_child]
        picker: TemplateChild<LanguagePicker>,

        #[property(get = Self::selected_language, set = Self::set_selected_language, nullable)]
        selected_language: PhantomData<Option<String>>,
        #[property(get = Self::languages, set = Self::set_languages)]
        languages: PhantomData<StrV>,
        init_locale: OnceCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LanguagePage {
        const NAME: &'static str = "SetupLanguagePage";
        type Type = super::LanguagePage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LanguagePage {
        fn constructed(&self) {
            self.parent_constructed();

            let init_locale = crate::locale();

            // Setup the welcome switcher
            for &locale in POPULAR_LOCALES {
                gettextrs::setlocale(gettextrs::LocaleCategory::LcMessages, locale);
                self.welcome.append(
                    &gtk::Label::builder()
                        .label(gettext("Welcome"))
                        .css_classes(["title-1"])
                        .hexpand(true)
                        .build(),
                );
            }
            gettextrs::setlocale(gettextrs::LocaleCategory::LcMessages, init_locale.clone());
            self.init_locale.set(init_locale).unwrap();
        }
    }

    impl WidgetImpl for LanguagePage {}
    impl NavigationPageImpl for LanguagePage {}
    impl BasePageImpl for LanguagePage {
        fn rebuild_widget(&self) -> glib::object::ObjectBuilder<'static, glib::Object> {
            let builder = self.parent_rebuild_widget();

            builder
                .property("selected-language", self.obj().selected_language())
                .property("languages", self.obj().languages())
        }
    }

    #[gtk::template_callbacks]
    impl LanguagePage {
        fn selected_language(&self) -> Option<String> {
            self.picker.selected()
        }

        fn set_selected_language(&self, language: Option<String>) {
            self.picker.set_selected(language);
        }

        fn languages(&self) -> StrV {
            self.picker.languages()
        }

        fn set_languages(&self, languages: StrV) {
            self.picker.set_languages(languages);
        }

        #[template_callback]
        fn on_shown(&self) {
            let obj = self.obj();
            *self.welcome_anim.borrow_mut() = Some(glib::timeout_add_local(
                Duration::new(8, 0),
                glib::clone!(
                    #[strong]
                    obj,
                    move || {
                        let welcome = &obj.imp().welcome;
                        welcome.scroll_to(&welcome.nth_page(1), true);
                        glib::ControlFlow::Continue
                    }
                ),
            ));
        }

        #[template_callback]
        fn on_hidden(&self) {
            if let Some(id) = self.welcome_anim.take() {
                id.remove();
            }
        }

        #[template_callback]
        fn on_welcome_changed(welcome: &adw::Carousel) {
            welcome.reorder(&welcome.nth_page(0), -1); // Infinite scrolling
        }

        #[template_callback]
        fn selected_changed(&self) {
            let Some(locale) = self.picker.selected() else {
                self.obj().notify("selected-language");
                return;
            };

            // TODO: Change to the default recommended keyboard layout, if known
            gettextrs::setlocale(gettextrs::LocaleCategory::LcMessages, locale);

            if self.init_locale.get().unwrap() != &crate::locale() {
                if let Some(window) = self.obj().root().and_downcast::<Window>() {
                    window.locale_changed();
                }
            }

            self.obj().notify("selected-language");
        }

        #[template_callback]
        fn languages_changed(&self) {
            self.obj().notify("languages");
        }

        #[template_callback]
        fn on_next(&self) {
            let Some(selected) = self.picker.selected() else {
                self.obj().go_subpage(Next::page());
                return;
            };

            glib::spawn_future_local(async move {
                let localed = match LocaledProxy::new(crate::system_bus()).await {
                    Ok(proxy) => proxy,
                    Err(e) => {
                        log::error!("Failed to obtain localed proxy: {e}");
                        return;
                    }
                };

                if let Err(e) = localed.set_locale(vec![selected], false).await {
                    log::error!("Failed to set localed locale: {e}");
                }
            });

            self.obj().go_subpage(Next::page());
        }
    }
}
