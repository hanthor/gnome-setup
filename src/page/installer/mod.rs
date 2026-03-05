/* SPDX-License-Identifier: GPL-3.0-or-later */

mod disk;
mod install;

use glib::object::Cast;

use crate::page::base::BasePage;
use disk::{DiskSelectPage, RstPage};

pub struct InstallerPage {}

impl InstallerPage {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> BasePage {
        if let Some(page) = RstPage::new() {
            page.upcast()
        } else {
            DiskSelectPage::new().upcast()
        }
    }
}
