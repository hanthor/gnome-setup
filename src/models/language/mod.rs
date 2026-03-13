/* SPDX-License-Identifier: GPL-3.0-or-later */

#[allow(clippy::module_inception)]
mod language;
mod language_model;

pub use language::{tokenize_and_casefold, Language};
pub use language_model::{LanguageModel, POPULAR_LOCALES};
