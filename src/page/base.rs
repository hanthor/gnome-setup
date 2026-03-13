/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;

use std::cell::Cell;
use std::future::IntoFuture;
use std::marker::PhantomData;
use std::sync::OnceLock;

use gettextrs::gettext;
use glib::subclass::Signal;

pub use imp::BasePagePropertiesExt;

glib::wrapper! {
    pub struct BasePage(ObjectSubclass<imp::BasePage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl BasePage {
    pub fn reload_locale(&self) {
        self.imp().reload_locale();
    }
}

pub trait BasePageExt: IsA<BasePage> + IsA<gtk::Widget> {
    fn go_next<F>(&self, next: F)
    where
        F: IntoFuture<Output = BasePage> + 'static,
    {
        // Project Convention: Types that are BasePages should have a associated function that
        // looks like:            pub async fn page(<any arguments>) -> BasePage
        // This should return either a newly-constructed Self, or call some other page's ::page()
        // function. This is how we handle the ordering of pages, conditional skipping, and so on.
        // This function should only ever called w/ the return value of one of these page
        // invocations.
        //
        // We're not using a trait to enforce this convention because some pages need additional
        // data passed into them at construction time. It would get needlessly messy

        let nav = self
            .ancestor(adw::NavigationView::static_type())
            .and_downcast::<adw::NavigationView>()
            .unwrap();

        glib::spawn_future_local(glib::clone!(
            #[strong(rename_to = curr)]
            self,
            #[strong]
            nav,
            async move {
                nav.set_cursor_from_name(Some("wait"));
                curr.set_sensitive(false);

                nav.push(&next.await);

                nav.set_cursor(None);
                curr.set_sensitive(true);
            }
        ));
    }
}

pub trait BasePageImpl: NavigationPageImpl {}

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

        fn new() -> Self {
            Self {
                can_next: Cell::new(true),
                can_close: Cell::new(true),
                default_clamp: Cell::new(true),
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

            self.reload_locale();
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

        pub(super) fn reload_locale(&self) {
            self.back_btn.set_tooltip_text(Some(&gettext("Back")));
            self.next_btn.set_tooltip_text(Some(&gettext("Next")));
            self.mobile_next_btn.set_label(&gettext("_Next"));
        }
    }
}

impl<O: IsA<BasePage> + IsA<gtk::Widget>> BasePageExt for O {}

unsafe impl<T: BasePageImpl> IsSubclassable<T> for BasePage {}
