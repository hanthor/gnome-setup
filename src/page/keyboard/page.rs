/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::base::{BasePage, BasePageExt, BasePageImpl, BasePageImplExt};
use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::collections::strv::StrV;

use crate::interfaces::LocaledProxy;
use crate::models::keyboard::Keyboard;

use super::picker::KeyboardPicker;

glib::wrapper! {
    pub struct KeyboardPage(ObjectSubclass<imp::KeyboardPage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl KeyboardPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_selected_keyboard_by_id(&self, id: Option<&str>) {
        self.imp().picker.set_selected_by_id(id);
    }
}

mod imp {
    use super::*;
    use std::marker::PhantomData;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/keyboard/page.blp")]
    #[properties(wrapper_type = super::KeyboardPage)]
    pub struct KeyboardPage {
        #[template_child]
        pub(super) picker: TemplateChild<KeyboardPicker>,
        #[property(get = Self::selected_keyboard, nullable)]
        selected_keyboard: PhantomData<Option<Keyboard>>,
        #[property(get = Self::keyboards, set = Self::set_keyboards)]
        keyboards: PhantomData<StrV>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyboardPage {
        const NAME: &'static str = "SetupKeyboardPage";
        type Type = super::KeyboardPage;
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
    impl ObjectImpl for KeyboardPage {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for KeyboardPage {}
    impl NavigationPageImpl for KeyboardPage {}
    impl BasePageImpl for KeyboardPage {
        fn rebuild_widget(&self) -> glib::object::ObjectBuilder<'static, glib::Object> {
            let builder = self.parent_rebuild_widget();

            builder
                .property("selected-keyboard", self.obj().selected_keyboard())
                .property("keyboards", self.obj().keyboards())
        }
    }

    #[gtk::template_callbacks]
    impl KeyboardPage {
        fn selected_keyboard(&self) -> Option<Keyboard> {
            self.picker.selected()
        }

        fn keyboards(&self) -> StrV {
            self.picker.keyboards()
        }

        fn set_keyboards(&self, keyboards: StrV) {
            self.picker.set_keyboards(keyboards);
        }

        #[template_callback]
        fn selected_changed(&self) {
            self.obj().notify("selected-keyboard");
        }

        #[template_callback]
        fn keyboards_changed(&self) {
            self.obj().notify("keyboards");
        }

        #[template_callback]
        fn on_next(&self) {
            let Some(keyboard) = self.picker.selected() else {
                self.obj().go_next();
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

                if let Err(e) = localed
                    .set_x11_keyboard(&keyboard.layout(), "", &keyboard.variant(), "", true, true)
                    .await
                {
                    log::error!("Failed to set localed keyboard: {e}");
                }
            });

            self.obj().go_next();
        }
    }
}
