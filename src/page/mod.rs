/* SPDX-License-Identifier: GPL-3.0-or-later */

mod base;
mod done;
mod eula;
mod installer;
mod language;
mod prerelease;
mod welcome;

pub use base::{BasePage, BasePageExt};
pub use done::DonePage;
pub use eula::EulaPage;
pub use language::LanguagePage;
pub use prerelease::PrereleasePage;
pub use welcome::WelcomePage;

#[allow(unused_imports)]
pub mod prelude {
    use super::*;

    pub use base::{BasePageExt, BasePagePropertiesExt};
}
