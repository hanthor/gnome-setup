/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;
use adw::subclass::prelude::*;
use appstream::prelude::*;

use std::sync::OnceLock;

use constcat::concat;
use gtk::glib;
use quick_xml::de::from_str;
use serde::{Deserialize, Deserializer};

use crate::config::PKGDATADIR;
use crate::page::base::{BasePage, BasePageExt, BasePageImpl};

const EULA_PATH: &str = concat!(PKGDATADIR, "/eula.xml");

glib::wrapper! {
    pub struct EulaPage(ObjectSubclass<imp::EulaPage>)
        @extends gtk::Widget, adw::NavigationPage, BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl EulaPage {
    pub fn new() -> Option<Self> {
        if read_xml().is_empty() {
            None
        } else {
            Some(glib::Object::new())
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/eula/page.blp")]
    pub struct EulaPage {
        #[template_child]
        pub eula: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EulaPage {
        const NAME: &'static str = "SetupEulaPage";
        type Type = super::EulaPage;
        type ParentType = BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EulaPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.load_eula();
        }
    }

    impl WidgetImpl for EulaPage {}
    impl NavigationPageImpl for EulaPage {}
    impl BasePageImpl for EulaPage {}

    #[gtk::template_callbacks]
    impl EulaPage {
        fn load_eula(&self) {
            let ctx = appstream::Context::new();
            ctx.set_locale(Some(&crate::locale()));

            let component = appstream::Component::new();
            component
                .load_from_bytes(&ctx, appstream::FormatKind::Xml, read_xml())
                .expect("AppStream component invalid");
            let agreement = component
                .agreement_by_kind(appstream::AgreementKind::Eula)
                .expect("AppStream component must have a EULA agreement");

            for section in agreement.sections() {
                let desc = section.description().expect("EULA section must have content");
                let markup: Vec<Markup> = from_str(&desc).unwrap();

                let section_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
                self.eula.append(&section_box);

                if let Some(name) = section.name() {
                    let header = gtk::Label::builder()
                        .label(name)
                        .wrap(true)
                        .xalign(0.0)
                        .css_classes(["title-3"])
                        .build();
                    section_box.append(&header);
                }

                for node in markup {
                    section_box.append(&node.widget());
                }
            }
        }

        #[template_callback]
        fn next(&self) {
            self.obj().go_next();
        }
    }
}

fn read_xml() -> &'static glib::Bytes {
    static ONCE: OnceLock<glib::Bytes> = OnceLock::new();
    ONCE.get_or_init(|| match std::fs::read(EULA_PATH) {
        Ok(bytes) => glib::Bytes::from_owned(bytes),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::error!("Failed to load {EULA_PATH}: {e}");
            }
            glib::Bytes::from_static(&[])
        }
    })
}

#[derive(Debug, Deserialize)]
enum Markup {
    #[serde(rename = "p")]
    Paragraph(#[serde(deserialize_with = "sanitize_spaces")] String),

    #[serde(rename = "ol")]
    OrderedList {
        #[serde(default, rename = "li", deserialize_with = "unwrap_li")]
        items: Vec<String>,
    },

    #[serde(rename = "ul")]
    UnorderedList {
        #[serde(default, rename = "li", deserialize_with = "unwrap_li")]
        items: Vec<String>,
    },
}

impl Markup {
    fn widget(&self) -> gtk::Widget {
        match self {
            Markup::Paragraph(text) => Self::label(text),
            Markup::OrderedList { items } => Self::list(true, items),
            Markup::UnorderedList { items } => Self::list(false, items),
        }
    }

    fn label(text: &str) -> gtk::Widget {
        gtk::Label::builder().label(text).xalign(0.0).wrap(true).build().upcast()
    }

    fn list(ordered: bool, items: &Vec<String>) -> gtk::Widget {
        let grid = gtk::Grid::builder().column_spacing(6).build();

        for (i, item) in (0i32..).zip(items) {
            let bullet = if ordered {
                (i + 1).to_string() + "."
            } else {
                "•".into()
            };
            let bullet = gtk::Label::builder().label(bullet).xalign(1.0).yalign(0.0).build();

            grid.attach(&bullet, 0, i, 1, 1);
            grid.attach(&Self::label(item), 1, i, 1, 1);
        }

        grid.upcast()
    }
}

fn sanitize_spaces<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(String::deserialize(deserializer)?
        .split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" "))
}

fn unwrap_li<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize, Debug)]
    struct ListItem(#[serde(deserialize_with = "sanitize_spaces")] String);
    Ok(Vec::<ListItem>::deserialize(deserializer)?.into_iter().map(|li| li.0).collect())
}
