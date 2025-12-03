//! These definitions are copied from `cargo_generate::config`, since that interface isn't pub.
//! Only keys we need are kept.

use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;

pub const CONFIG_FILE_NAME: &str = "cargo-generate.toml";

#[derive(Deserialize, Debug, PartialEq, Default, Clone)]
pub struct Config {
    pub template: Option<TemplateConfig>,
    pub placeholders: Option<TemplateSlotsTable>,
    pub conditional: Option<HashMap<String, ConditionalConfig>>,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Default, Clone)]
pub struct TemplateConfig {
    pub sub_templates: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct ConditionalConfig {
    pub placeholders: Option<TemplateSlotsTable>,
}

#[derive(Deserialize, Debug, PartialEq, Clone, Default)]
pub struct TemplateSlotsTable(pub IndexMap<String, toml::Value>);
