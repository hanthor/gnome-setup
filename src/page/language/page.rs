/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::base::{BasePage, BasePageExt, BasePageImpl, BasePagePropertiesExt};
use adw::prelude::*;
use adw::subclass::prelude::*;

use std::cell::RefCell;
use std::time::Duration;

use gettextrs::gettext;

use crate::interfaces::LocaledProxy;
use crate::models::language::{Language, POPULAR_LOCALES};

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

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/language/page.blp")]
    pub struct LanguagePage {
        #[template_child]
        welcome: TemplateChild<adw::Carousel>,
        welcome_anim: RefCell<Option<glib::SourceId>>,

        #[template_child]
        picker: TemplateChild<LanguagePicker>,
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

    impl ObjectImpl for LanguagePage {
        fn constructed(&self) {
            self.parent_constructed();

            // Populate UI w/ initial language
            self.load_locale();

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
            gettextrs::setlocale(gettextrs::LocaleCategory::LcMessages, "");

            // Pre-select the locale if already set by something
            self.picker.set_selected(Language::system_default_id());
        }
    }

    impl WidgetImpl for LanguagePage {}
    impl NavigationPageImpl for LanguagePage {}
    impl BasePageImpl for LanguagePage {}

    #[gtk::template_callbacks]
    impl LanguagePage {
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

        fn load_locale(&self) {
            self.obj().upcast_ref::<BasePage>().reload_locale();

            self.obj().set_title(&gettext("Language"));
            self.picker.reload_locale();
        }

        #[template_callback]
        fn on_selected(&self, locale: Option<String>) {
            self.obj().set_ready(locale.is_some());

            if let Some(locale) = locale {
                gettextrs::setlocale(gettextrs::LocaleCategory::LcMessages, locale);
                self.load_locale();

                // TODO: Change to the default recommended keyboard layout, if known
            }
        }

        #[template_callback]
        fn on_next(&self) {
            let selected = self.picker.selected().unwrap();

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
