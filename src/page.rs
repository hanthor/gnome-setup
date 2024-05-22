/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use glib::subclass::Signal;

use std::{cell::Cell, marker::PhantomData};
use std::sync::OnceLock;

mod prerelease;
mod eula;
mod demo;

pub use prerelease::PrereleasePage;
pub use eula::EulaPage;
pub use demo::DemoPage;

glib::wrapper! {
    pub struct BasePage(ObjectSubclass<imp::BasePage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for BasePage {
        const NAME: &'static str = "SetupBasePage";
        const ABSTRACT: bool = true;

        type Type = super::BasePage;
        type ParentType = adw::NavigationPage;

        fn new() -> Self {
            Self {
                can_next: Cell::new(true),
                ..Default::default()
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page.blp")]
    #[properties(wrapper_type = super::BasePage)]
    pub struct BasePage {
        #[property(get = BasePage::get_child, set = BasePage::set_child)]
        child: PhantomData<Option<gtk::Widget>>,

        #[property(get, set)]
        can_next: Cell<bool>,
        #[property(get, set)]
        ready: Cell<bool>,

        #[template_child]
        pub contents: TemplateChild<adw::Bin>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for BasePage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![
                Signal::builder("next").build(),
            ])
        }
    }

    impl WidgetImpl for BasePage {}
    impl NavigationPageImpl for BasePage {}

    #[gtk::template_callbacks]
    impl BasePage {
        fn get_child(&self) -> Option<gtk::Widget> {
            self.contents.get().child()
        }

        fn set_child(&self, value: Option<&gtk::Widget>) {
            self.contents.get().set_child(value);
        }

        #[template_callback]
        fn on_next(&self) {
            self.obj().emit_by_name::<()>("next", &[]);
        }

        pub(super) fn go_next(&self, page: &super::BasePage) {
            let nav_view = self
                .obj()
                .ancestor(adw::NavigationView::static_type())
                .and_downcast::<adw::NavigationView>()
                .unwrap();
            nav_view.push(page);
        }
    }
}

pub trait BasePageExt: IsA<BasePage> {
    fn go_next(&self, next: impl IsA<BasePage>) {
        self.upcast_ref::<BasePage>().imp().go_next(next.upcast_ref());
    }
}

impl<O: IsA<BasePage>> BasePageExt for O {}

pub trait BasePageImpl: NavigationPageImpl {}

unsafe impl<T: BasePageImpl> IsSubclassable<T> for BasePage {}
