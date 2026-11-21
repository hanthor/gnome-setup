/* SPDX-License-Identifier: GPL-3.0-or-later */

mod base;
mod done;
mod eula;
mod installer;
mod keyboard;
mod language;
mod passphrase;
mod prerelease;
mod recovery_key;
mod welcome;

pub use base::{BasePage, BasePageExt};
pub use done::DonePage;
pub use eula::EulaPage;
pub use installer::InstallerPage;
pub use keyboard::KeyboardPage;
pub use language::LanguagePage;
pub use passphrase::PassphrasePage;
pub use prerelease::PrereleasePage;
pub use recovery_key::RecoveryKeyPage;
pub use welcome::WelcomePage;

#[allow(unused_imports)]
pub mod prelude {
    use super::*;

    pub use base::{BasePageExt, BasePagePropertiesExt};
}
