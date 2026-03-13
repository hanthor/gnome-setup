/* SPDX-License-Identifier: GPL-3.0-or-later */

#[allow(clippy::module_inception)]
mod disk;
mod disks_model;

pub use disk::{Disk, DiskStatus, InstallPhase};
pub use disks_model::DisksModel;
