/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::object::ObjectBuilder;

use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::OnceLock;

use glib::subclass::Signal;

pub use imp::BasePagePropertiesExt;

glib::wrapper! {
    pub struct BasePage(ObjectSubclass<imp::BasePage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

pub trait BasePageExt: IsA<BasePage> {
    /// Convenience method to add a new subpage to the `adw::NavigationView`
    fn go_subpage(&self, page: impl IsA<BasePage>) {
        let nav = self
            .upcast_ref::<BasePage>()
            .ancestor(adw::NavigationView::static_type())
            .and_downcast::<adw::NavigationView>()
            .unwrap();

        nav.push(&page.upcast());
    }

    fn rebuild(&self) -> BasePage {
        let this = self.upcast_ref();
        let class = this.class();

        (class.as_ref().rebuild_widget)(this).build().downcast().unwrap()
    }
}

pub trait BasePageImplExt: BasePageImpl {
    fn parent_rebuild_widget(&self) -> ObjectBuilder<'static, glib::Object> {
        let data = Self::type_data();
        let parent_class = unsafe { &*(data.as_ref().parent_class() as *const ffi::Class) };
        let rebuild_widget = parent_class.rebuild_widget;

        unsafe { rebuild_widget(self.obj().unsafe_cast_ref()) }
    }
}

impl<T: IsA<BasePage>> BasePageExt for T {}
impl<T: BasePageImpl> BasePageImplExt for T {}

pub trait BasePageImpl: NavigationPageImpl + ObjectSubclass<Type: IsA<BasePage>> {
    fn rebuild_widget(&self) -> ObjectBuilder<'static, glib::Object> {
        self.parent_rebuild_widget()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "src/page/base.blp")]
    #[properties(wrapper_type = super::BasePage, ext_trait)]
    pub struct BasePage {
        #[property(get = BasePage::get_child, set = BasePage::set_child)]
        child: PhantomData<Option<gtk::Widget>>,
        #[property(get = BasePage::get_bottom_child, set = BasePage::set_bottom_child)]
        bottom_child: PhantomData<Option<gtk::Widget>>,

        #[property(get, set)]
        can_next: Cell<bool>,
        #[property(get, set)]
        ready: Cell<bool>,
        #[property(get, set)]
        can_close: Cell<bool>,

        #[property(get, set)]
        default_clamp: Cell<bool>,
        #[property(get, set)]
        big_title: Cell<bool>,

        #[template_child]
        headerbar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        viewport: TemplateChild<gtk::Viewport>,
        #[template_child]
        contents: TemplateChild<adw::Bin>,
        #[template_child]
        bottom_contents: TemplateChild<adw::Bin>,

        #[template_child]
        back_btn: TemplateChild<gtk::Button>,
        #[template_child]
        next_btn: TemplateChild<gtk::Button>,
        #[template_child]
        mobile_next_btn: TemplateChild<gtk::Button>,

        #[template_child]
        toolbarview: TemplateChild<adw::ToolbarView>,
        #[property(get = BasePage::get_top_bar_height)]
        top_bar_height: PhantomData<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BasePage {
        const NAME: &'static str = "SetupBasePage";
        const ABSTRACT: bool = true;

        type Type = super::BasePage;
        type ParentType = adw::NavigationPage;
        type Class = super::ffi::Class;

        fn new() -> Self {
            Self {
                can_next: Cell::new(true),
                can_close: Cell::new(true),
                default_clamp: Cell::new(true),
                ..Default::default()
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.rebuild_widget = |obj| obj.imp().rebuild_widget_default();

            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl BasePage {
        fn rebuild_widget_default(&self) -> ObjectBuilder<'static, glib::Object> {
            glib::Object::builder_with_type(self.obj().type_())
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for BasePage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("next").build()])
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let adj = self.viewport.vadjustment().unwrap();
            adj.connect_closure(
                "value-changed",
                false,
                glib::closure_local!(
                    #[watch]
                    obj,
                    move |adj: gtk::Adjustment| {
                        obj.imp().headerbar.set_show_title(adj.value() > 0.0);
                    }
                ),
            );
        }
    }

    impl WidgetImpl for BasePage {}

    impl NavigationPageImpl for BasePage {
        fn showing(&self) {
            let window = self
                .obj()
                .ancestor(gtk::Window::static_type())
                .and_downcast::<gtk::Window>()
                .unwrap();
            window.set_deletable(self.can_close.get() && crate::in_session());
        }
    }

    #[gtk::template_callbacks]
    impl BasePage {
        fn get_child(&self) -> Option<gtk::Widget> {
            self.contents.child()
        }

        fn set_child(&self, value: Option<gtk::Widget>) {
            let child = match value {
                Some(child) if self.default_clamp.get() => Some(
                    adw::Clamp::builder()
                        .child(&child)
                        .maximum_size(450)
                        .tightening_threshold(200)
                        .build()
                        .upcast(),
                ),
                _ => value,
            };
            self.contents.set_child(child.as_ref());
        }

        fn get_bottom_child(&self) -> Option<gtk::Widget> {
            self.bottom_contents.child()
        }

        fn set_bottom_child(&self, value: Option<&gtk::Widget>) {
            self.bottom_contents.set_child(value);
            self.bottom_contents.set_visible(value.is_some());
        }

        #[template_callback]
        fn on_next(&self) {
            self.obj().emit_by_name::<()>("next", &[]);
        }

        #[template_callback]
        fn toolbar_height_changed(&self) {
            self.obj().notify_top_bar_height();
        }

        fn get_top_bar_height(&self) -> i32 {
            self.toolbarview.top_bar_height()
        }
    }
}

unsafe impl<Obj: BasePageImpl> IsSubclassable<Obj> for BasePage {
    fn class_init(class: &mut glib::Class<Self>) {
        Self::parent_class_init::<Obj>(class);

        let klass = class.as_mut();

        klass.rebuild_widget = |obj| {
            let this = unsafe { obj.unsafe_cast_ref::<<Obj as ObjectSubclass>::Type>().imp() };
            BasePageImpl::rebuild_widget(this)
        };
    }
}

#[allow(dead_code)]
pub mod ffi {
    use glib::object::ObjectBuilder;

    pub type Instance = <super::BasePage as super::ObjectSubclass>::Instance;

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct Class {
        parent_class: adw::ffi::AdwNavigationPageClass,
        pub(super) rebuild_widget: fn(&super::BasePage) -> ObjectBuilder<'static, glib::Object>,
    }

    unsafe impl glib::subclass::types::ClassStruct for Class {
        type Type = super::imp::BasePage;
    }

    impl std::ops::Deref for Class {
        type Target = adw::ffi::AdwNavigationPageClass;

        fn deref(&self) -> &Self::Target {
            &self.parent_class
        }
    }
}
