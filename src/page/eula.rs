/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::subclass::prelude::*;
use gtk::glib;

use crate::page::{self, BasePageImpl, BasePageExt};

glib::wrapper! {
    pub struct EulaPage(ObjectSubclass<imp::EulaPage>)
        @extends gtk::Widget, adw::NavigationPage, page::BasePage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

mod imp {
    use super::*;

    #[glib::object_subclass]
    impl ObjectSubclass for EulaPage {
        const NAME: &'static str = "SetupEulaPage";
        type Type = super::EulaPage;
        type ParentType = page::BasePage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/page/eula/eula.blp")]
    pub struct EulaPage {
        #[template_child]
        pub eula: TemplateChild<gtk::Label>,
    }

    impl ObjectImpl for EulaPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.eula.set_label(r#"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Id ornare arcu odio ut. Ut venenatis tellus in metus. Eget gravida cum sociis natoque penatibus et magnis. Eros donec ac odio tempor orci dapibus. In ornare quam viverra orci. Dignissim diam quis enim lobortis scelerisque fermentum dui. Orci porta non pulvinar neque laoreet. Nec ultrices dui sapien eget. Odio facilisis mauris sit amet massa vitae. Gravida cum sociis natoque penatibus et magnis dis parturient. Vitae elementum curabitur vitae nunc sed velit dignissim sodales ut. Feugiat vivamus at augue eget arcu dictum.\n
Imperdiet massa tincidunt nunc pulvinar sapien et. Mattis ullamcorper velit sed ullamcorper morbi. Sit amet dictum sit amet justo. Ornare arcu dui vivamus arcu felis bibendum ut tristique et. Dictum sit amet justo donec enim. Fermentum dui faucibus in ornare quam. Ut diam quam nulla porttitor massa id neque aliquam vestibulum. Ut porttitor leo a diam sollicitudin tempor id eu. Integer vitae justo eget magna fermentum iaculis eu. Nunc scelerisque viverra mauris in aliquam sem.\n
Accumsan tortor posuere ac ut consequat semper. Ut ornare lectus sit amet est placerat in egestas erat. Dignissim suspendisse in est ante in. Faucibus pulvinar elementum integer enim. Massa ultricies mi quis hendrerit dolor. In est ante in nibh mauris. Amet massa vitae tortor condimentum lacinia quis vel eros. Diam maecenas ultricies mi eget mauris. Convallis convallis tellus id interdum velit laoreet id donec. Massa tincidunt nunc pulvinar sapien et ligula. Ipsum dolor sit amet consectetur adipiscing elit duis. Sit amet consectetur adipiscing elit. Ipsum consequat nisl vel pretium. Viverra suspendisse potenti nullam ac tortor vitae purus faucibus. Diam volutpat commodo sed egestas egestas fringilla. Facilisis mauris sit amet massa vitae tortor condimentum. Non quam lacus suspendisse faucibus interdum. Nisl tincidunt eget nullam non nisi. Faucibus purus in massa tempor nec feugiat nisl pretium.\n
Est velit egestas dui id ornare arcu odio ut. Tempus imperdiet nulla malesuada pellentesque elit eget gravida cum sociis. Scelerisque varius morbi enim nunc faucibus. Vulputate dignissim suspendisse in est ante in nibh mauris cursus. Libero justo laoreet sit amet cursus sit amet dictum sit. Amet consectetur adipiscing elit pellentesque habitant morbi tristique. Vel pretium lectus quam id leo in. Faucibus a pellentesque sit amet porttitor eget. Etiam sit amet nisl purus in mollis nunc. In nisl nisi scelerisque eu ultrices vitae auctor eu augue. In nulla posuere sollicitudin aliquam ultrices. Aliquet sagittis id consectetur purus ut faucibus pulvinar elementum. Bibendum est ultricies integer quis auctor. Velit sed ullamcorper morbi tincidunt ornare massa eget. Eu tincidunt tortor aliquam nulla facilisi cras fermentum odio. Potenti nullam ac tortor vitae. Est ante in nibh mauris cursus mattis. Mauris rhoncus aenean vel elit scelerisque mauris pellentesque. Placerat vestibulum lectus mauris ultrices eros in cursus turpis.\n
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Id ornare arcu odio ut. Ut venenatis tellus in metus. Eget gravida cum sociis natoque penatibus et magnis. Eros donec ac odio tempor orci dapibus. In ornare quam viverra orci. Dignissim diam quis enim lobortis scelerisque fermentum dui. Orci porta non pulvinar neque laoreet. Nec ultrices dui sapien eget. Odio facilisis mauris sit amet massa vitae. Gravida cum sociis natoque penatibus et magnis dis parturient. Vitae elementum curabitur vitae nunc sed velit dignissim sodales ut. Feugiat vivamus at augue eget arcu dictum.\n
Imperdiet massa tincidunt nunc pulvinar sapien et. Mattis ullamcorper velit sed ullamcorper morbi. Sit amet dictum sit amet justo. Ornare arcu dui vivamus arcu felis bibendum ut tristique et. Dictum sit amet justo donec enim. Fermentum dui faucibus in ornare quam. Ut diam quam nulla porttitor massa id neque aliquam vestibulum. Ut porttitor leo a diam sollicitudin tempor id eu. Integer vitae justo eget magna fermentum iaculis eu. Nunc scelerisque viverra mauris in aliquam sem.\n
Accumsan tortor posuere ac ut consequat semper. Ut ornare lectus sit amet est placerat in egestas erat. Dignissim suspendisse in est ante in. Faucibus pulvinar elementum integer enim. Massa ultricies mi quis hendrerit dolor. In est ante in nibh mauris. Amet massa vitae tortor condimentum lacinia quis vel eros. Diam maecenas ultricies mi eget mauris. Convallis convallis tellus id interdum velit laoreet id donec. Massa tincidunt nunc pulvinar sapien et ligula. Ipsum dolor sit amet consectetur adipiscing elit duis. Sit amet consectetur adipiscing elit. Ipsum consequat nisl vel pretium. Viverra suspendisse potenti nullam ac tortor vitae purus faucibus. Diam volutpat commodo sed egestas egestas fringilla. Facilisis mauris sit amet massa vitae tortor condimentum. Non quam lacus suspendisse faucibus interdum. Nisl tincidunt eget nullam non nisi. Faucibus purus in massa tempor nec feugiat nisl pretium.\n
Est velit egestas dui id ornare arcu odio ut. Tempus imperdiet nulla malesuada pellentesque elit eget gravida cum sociis. Scelerisque varius morbi enim nunc faucibus. Vulputate dignissim suspendisse in est ante in nibh mauris cursus. Libero justo laoreet sit amet cursus sit amet dictum sit. Amet consectetur adipiscing elit pellentesque habitant morbi tristique. Vel pretium lectus quam id leo in. Faucibus a pellentesque sit amet porttitor eget. Etiam sit amet nisl purus in mollis nunc. In nisl nisi scelerisque eu ultrices vitae auctor eu augue. In nulla posuere sollicitudin aliquam ultrices. Aliquet sagittis id consectetur purus ut faucibus pulvinar elementum. Bibendum est ultricies integer quis auctor. Velit sed ullamcorper morbi tincidunt ornare massa eget. Eu tincidunt tortor aliquam nulla facilisi cras fermentum odio. Potenti nullam ac tortor vitae. Est ante in nibh mauris cursus mattis. Mauris rhoncus aenean vel elit scelerisque mauris pellentesque. Placerat vestibulum lectus mauris ultrices eros in cursus turpis."#);
        }
    }

    impl WidgetImpl for EulaPage {}
    impl NavigationPageImpl for EulaPage {}
    impl BasePageImpl for EulaPage {}
}

#[gtk::template_callbacks]
impl EulaPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn next(&self) {
        // TODO: Mockup
        self.go_next(page::InstallPage::new());
    }
}
