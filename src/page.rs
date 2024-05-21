/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use std::{cell::Cell, marker::PhantomData};

mod prerelease;
mod eula;

pub use prerelease::PrereleasePage;
pub use eula::EulaPage;

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
        type Class = super::Class;

        fn new() -> Self {
            Self {
                can_next: Cell::new(true),
                ..Default::default()
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.next = |_| { panic!() };
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
    impl ObjectImpl for BasePage {}

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
            let nav_view = self
                .obj()
                .ancestor(adw::NavigationView::static_type())
                .and_downcast::<adw::NavigationView>()
                .unwrap();
            let next_page = self.obj().next();
            nav_view.push(&next_page);
        }
    }
}

pub trait BasePageExt: IsA<BasePage> {
    fn next(&self) -> BasePage {
        let obj = self.upcast_ref::<BasePage>();
        (obj.class().as_ref().next)(obj)
    }
}

impl<O: IsA<BasePage>> BasePageExt for O {}

pub trait BasePageImpl: NavigationPageImpl {
    fn next(&self) -> BasePage;
}

unsafe impl<T: BasePageImpl> IsSubclassable<T> for BasePage {
    fn class_init(class: &mut glib::Class<Self>) {
        Self::parent_class_init::<T>(class.upcast_ref_mut());

        let klass = class.as_mut();
        klass.next = |obj| unsafe {
            let imp = obj.unsafe_cast_ref::<T::Type>().imp();
            imp.next()
        }
    }
}

#[repr(C)]
pub struct Class {
    pub parent_class: adw::ffi::AdwNavigationPageClass,
    pub next: fn(&BasePage) -> BasePage,
}

// Unsafe to enforce that Class is #[repr(C)]
unsafe impl ClassStruct for Class {
    type Type = imp::BasePage;
}

// Deref directly to the parent class' class struct.
impl std::ops::Deref for Class {
    type Target = glib::Class<adw::NavigationPage>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(&self.parent_class as *const _ as *const _) }
    }
}
