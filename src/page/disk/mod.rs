/* SPDX-License-Identifier: GPL-3.0-or-later */

mod confirm;
mod disk_row;
mod model;
mod obj;
mod rst;
mod select;
mod wipe_or_free;

// The entry-point page to all the disk pages
pub struct DiskPages;

impl DiskPages {
    pub async fn page() -> crate::page::BasePage {
        rst::RstPage::page().await
    }

    pub fn preload() {
        model::DisksModel::default(); // Constructing the model starts loading it
    }
}
