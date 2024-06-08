/* SPDX-License-Identifier: GPL-3.0-or-later */

mod confirm;
mod no_disks;
mod rst;
mod select;
mod wipe_or_free;

// The entry-point page to all the disk pages
pub type DiskPages = rst::RstPage;
