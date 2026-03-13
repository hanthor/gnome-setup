/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::gio;

use crate::models::language::LanguageModel;
use crate::page::{BasePage, BasePageExt, LanguagePage, WelcomePage};

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
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/window.blp")]
    pub struct Window {
        #[template_child]
        pub nav: TemplateChild<adw::NavigationView>,
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

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = this)]
                self,
                async move {
                    let page: BasePage = if crate::in_session() {
                        WelcomePage::page()
                    } else {
                        LanguagePage::page()
                    };

                    page.set_can_pop(false);
                    this.nav.push(&page);
                }
            ));
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}
