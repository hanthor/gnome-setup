use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib};

mod imp {
    use super::*;

    use std::cell::OnceCell;

    use gtk::graphene;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::QRCodePaintable)]
    pub struct QRCodePaintable {
        #[property(get, set, construct_only)]
        pub data: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub color: OnceCell<gdk::RGBA>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for QRCodePaintable {
        const NAME: &'static str = "SetupQRCodePaintable";
        type Type = super::QRCodePaintable;
        type Interfaces = (gdk::Paintable,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for QRCodePaintable {}

    impl PaintableImpl for QRCodePaintable {
        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            let snapshot = snapshot.downcast_ref::<gtk::Snapshot>().unwrap();
            let qr =
                qrcode::QrCode::new(self.data.get().unwrap()).expect("Unable to create qr code");
            let stride = qr.width();
            let colors = qr.into_colors();

            let pixel_width = width as f32 / stride as f32;
            let pixel_height = height as f32 / stride as f32;
            let color = self.color.get().unwrap();

            for (i, value) in colors.iter().enumerate() {
                if value == &qrcode::Color::Dark {
                    let x = i.div_euclid(stride);
                    let y = i - x * stride;
                    let position = graphene::Rect::new(
                        x as f32 * pixel_width,
                        y as f32 * pixel_height,
                        pixel_width,
                        pixel_height,
                    );

                    snapshot.append_color(&color, &position);
                }
            }
        }

        fn flags(&self) -> gdk::PaintableFlags {
            gdk::PaintableFlags::SIZE | gdk::PaintableFlags::CONTENTS
        }
    }
}

glib::wrapper! {
    pub struct QRCodePaintable(ObjectSubclass<imp::QRCodePaintable>)
        @implements gdk::Paintable;
}

impl QRCodePaintable {
    pub fn new(data: &str, color: gdk::RGBA) -> Self {
        glib::Object::builder().property("data", data).property("color", color).build()
    }
}
