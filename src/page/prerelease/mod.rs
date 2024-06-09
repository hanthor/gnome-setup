/* SPDX-License-Identifier: GPL-3.0-or-later */

use crate::page::subclass::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use gettextrs::gettext;
use gtk::glib;

use crate::osinfo;
use crate::page::EulaPage as Next;

glib::wrapper! {
    pub struct PrereleasePage(ObjectSubclass<imp::PrereleasePage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// https://github.com/gtk-rs/gtk-rs-core/issues/658
impl Default for PrereleasePage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl PrereleasePage {
    pub async fn page() -> BasePage {
        use osinfo::ReleaseType as R;
        match osinfo::release_type() {
            R::PreRelease | R::Experimental { .. } => Self::default().upcast(),
            R::Stable | R::Lts => Next::page().await,
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/prerelease/page.blp")]
    pub struct PrereleasePage {
        #[template_child]
        status: TemplateChild<adw::StatusPage>,
        #[template_child]
        experiment: TemplateChild<adw::ActionRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PrereleasePage {
        const NAME: &'static str = "SetupPrereleasePage";
        type Type = super::PrereleasePage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PrereleasePage {
        fn constructed(&self) {
            self.parent_constructed();

            use osinfo::ReleaseType as R;
            match osinfo::release_type() {
                R::Stable | R::Lts => unreachable!(), // Would have skipped this page entirely
                R::PreRelease => {
                    self.status.set_icon_name(Some("dialog-warning-symbolic"));
                    self.status.set_title(&gettext("Pre-release Software"));
                    self.status.set_description(Some(&gettext!(
                        "This is a pre-release build of {}. Please be aware that breaking changes may \
                        still occur. You may need to manually migrate data after such a change. We \
                        provide no stability or security guarantees. Back up all important data. Do \
                        not use in a production environment.",
                        osinfo::name(),
                    )));
                    self.status.add_css_class("prerelease");
                }
                R::Experimental { description } => {
                    self.status.set_icon_name(Some("experimental-symbolic"));
                    self.status.set_title(&gettext("Experimental Software"));
                    self.status.set_description(Some(&gettext!(
                        "This is an experimental build of {}, built to test new features that are \
                        still in development. Breaking changes and data loss are likely. We provide \
                        no stability or security guarantees. Do not store any important data. Do not \
                        use in a production environment.",
                        osinfo::name(),
                    )));
                    self.status.add_css_class("experimental");

                    if let Some(description) = description {
                        self.experiment.set_visible(true);
                        self.experiment.set_subtitle(description);
                    }
                }
            }
        }
    }

    impl WidgetImpl for PrereleasePage {}
    impl NavigationPageImpl for PrereleasePage {}
    impl BasePageImpl for PrereleasePage {}

    #[gtk::template_callbacks]
    impl PrereleasePage {
        #[template_callback]
        fn next(&self) {
            self.obj().go_next(Next::page());
        }
    }
}
