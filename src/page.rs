/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use std::{cell::Cell, marker::PhantomData};

mod prerelease;
pub use prerelease::SetupPrereleasePage;

mod eula;
pub use eula::SetupEulaPage;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page.blp")]
    #[properties(wrapper_type = super::SetupPage)]
    pub struct SetupPage {
        #[property(get = SetupPage::get_child, set = SetupPage::set_child)]
        child: PhantomData<Option<gtk::Widget>>,

        #[property(get, set)]
        can_next: Cell<bool>,
        #[property(get, set)]
        ready: Cell<bool>,

        #[template_child]
        pub contents: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupPage {
        const NAME: &'static str = "SetupPage";
        type Type = super::SetupPage;
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
            klass.bind_template_instance_callbacks();

            klass.next = |_| { panic!() };
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SetupPage {}

    impl WidgetImpl for SetupPage {}
    impl NavigationPageImpl for SetupPage {}

    impl SetupPage {
        fn get_child(&self) -> Option<gtk::Widget> {
            self.contents.get().child()
        }

        fn set_child(&self, value: Option<&gtk::Widget>) {
            self.contents.get().set_child(value);
        }
    }
}

glib::wrapper! {
    pub struct SetupPage(ObjectSubclass<imp::SetupPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl SetupPage {
    #[template_callback]
    fn on_next(&self) {
        let next_page = self.next();
        let nav = self
            .ancestor(adw::NavigationView::static_type())
            .and_downcast::<adw::NavigationView>()
            .unwrap();
        nav.push(&next_page);
    }
}

pub trait SetupPageExt: IsA<SetupPage> {
    fn next(&self) -> SetupPage {
        let obj = self.upcast_ref::<SetupPage>();
        (obj.class().as_ref().next)(obj)
    }
}

impl<O: IsA<SetupPage>> SetupPageExt for O {}

pub trait SetupPageImpl: NavigationPageImpl {
    fn next(&self) -> SetupPage;
}

unsafe impl<T: SetupPageImpl> IsSubclassable<T> for SetupPage {
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
    pub next: fn(&SetupPage) -> SetupPage,
}

// Unsafe to enforce that Class is #[repr(C)]
unsafe impl ClassStruct for Class {
    type Type = imp::SetupPage;
}

// Deref directly to the parent class' class struct.
impl std::ops::Deref for Class {
    type Target = glib::Class<adw::NavigationPage>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(&self.parent_class as *const _ as *const _) }
    }
}
