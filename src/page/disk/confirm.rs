/* SPDX-License-Identifier: GPL-3.0-or-later */

use adw::prelude::*;

use gettextrs::gettext;

fn base_expected_dialog() -> adw::AlertDialog {
    let dialog = adw::AlertDialog::builder()
        .heading(&gettext("Begin Installation?"))
        .default_response("confirm")
        .close_response("cancel")
        .build();

    dialog.add_response("cancel", &gettext("Cancel"));

    dialog
}

pub async fn nondestructive(parent: &impl IsA<gtk::Widget>) -> bool {
    let dialog = base_expected_dialog();

    dialog.set_body(&gettext("Once the installation begins, the changes to the selected disk are irreversible. Continue with installation?"));

    dialog.add_response("confirm", &gettext("Install"));
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);

    dialog.choose_future(parent).await == "confirm"
}

pub async fn expected_wipe(parent: &impl IsA<gtk::Widget>) -> bool {
    let dialog = base_expected_dialog();

    dialog.set_body(&gettext("Proceeding will fully erase all software, documents, and data from the disk. Continue with installation?"));

    dialog.add_response("confirm", &gettext("Erase and Install"));
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);

    dialog.choose_future(parent).await == "confirm"
}

fn base_surprise_dialog() -> adw::AlertDialog {
    let dialog = adw::AlertDialog::builder()
        .body_use_markup(true)
        .default_response("cancel")
        .close_response("cancel")
        .build();

    dialog.add_response("cancel", &gettext("Cancel"));

    let confirm_text = gettext("Erase and Install");
    let mut countdown = 3;
    dialog.add_response("confirm", &format!("{confirm_text} ({countdown})"));
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
    dialog.set_response_enabled("confirm", false);

    glib::timeout_add_local(
        std::time::Duration::new(1, 0),
        glib::clone!(@strong dialog => move || {
            countdown -= 1;
            if countdown > 0 {
                dialog.set_response_label("confirm", &format!("{confirm_text} ({countdown})"));
                glib::ControlFlow::Continue
            } else {
                dialog.set_response_label("confirm", &confirm_text);
                dialog.set_response_enabled("confirm", true);
                glib::ControlFlow::Break
            }
        }),
    );

    dialog
}

pub async fn insufficient_wipe(parent: &impl IsA<gtk::Widget>) -> bool {
    let dialog = base_surprise_dialog();
    dialog.set_heading(Some(&gettext("Insufficient Free Space")));
    dialog.set_body(&gettext("There is not enough free space on the selected disk to install the operating system. <b>Proceeding will fully erase all software, documents, and data from the disk.</b> Continue with installation?"));
    dialog.choose_future(parent).await == "confirm"
}

pub async fn full_wipe(parent: &impl IsA<gtk::Widget>) -> bool {
    let dialog = base_surprise_dialog();
    dialog.set_heading(Some(&gettext("Disk is Full")));
    dialog.set_body(&gettext("The selected disk has no free space to install the operating system. <b>Proceeding will fully erase all software, documents, and data from the disk.</b> Continue with installation?"));
    dialog.choose_future(parent).await == "confirm"
}
