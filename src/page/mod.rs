/* SPDX-License-Identifier: GPL-3.0-or-later */

mod base;
mod disk;
mod eula;
mod install;
mod language;
mod prerelease;
mod welcome;

pub use base::BasePage;
pub use disk::DiskPages;
pub use eula::EulaPage;
pub use install::InstallPage;
pub use language::LanguagePage;
pub use prerelease::PrereleasePage;
pub use welcome::WelcomePage;

#[allow(unused_imports)]
pub mod prelude {
    use super::*;

    pub use base::{BasePageExt, BasePagePropertiesExt};
}
