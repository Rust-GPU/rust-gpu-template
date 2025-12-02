use cargo_generate::GenerateArgs;
use clap::Parser;
use clap::builder::PossibleValue;
use log::debug;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use strum::{Display, EnumString, IntoStaticStr, VariantArray};

/// All possible placeholder *values*.
///
/// We assume there are no duplicate values for placeholders, so we don't need to type out the key / placeholder name,
/// but can derive the key from the value directly.
#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Display, EnumString, IntoStaticStr, VariantArray)]
pub enum Values {
    #[strum(to_string = "cargo-gpu")]
    CargoGpu,
    #[strum(to_string = "spirv-builder")]
    SpirvBuilder,
}

impl Debug for Values {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl clap::ValueEnum for Values {
    fn value_variants<'a>() -> &'a [Self] {
        Values::VARIANTS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(self.value()))
    }
}

impl Values {
    pub fn key(&self) -> Placeholders {
        match self {
            Values::CargoGpu | Values::SpirvBuilder => Placeholders::IntegrationType,
        }
    }

    pub fn value(&self) -> &'static str {
        <&'static str>::from(self)
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Display, EnumString, IntoStaticStr, VariantArray)]
pub enum Placeholders {
    #[strum(to_string = "integration_type")]
    IntegrationType,
}

impl Debug for Placeholders {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

#[derive(Parser, Debug, Default)]
pub struct Generate {
    /// Directory where to place the generated templates
    #[clap(long)]
    out: Option<PathBuf>,
    values: Vec<Values>,
}

impl Generate {
    /// Computes all template variants to expand to and returns them in a Vec.
    /// The inner vec is guaranteed to contain no duplicate [`Placeholders`] and is sorted by order of the
    /// [`Placeholders`] in the enum.
    fn variants(&self) -> Vec<Vec<Values>> {
        // insert all Placeholders with empty Vec for possible values
        let mut variant_map: HashMap<Placeholders, Vec<Values>> = Placeholders::VARIANTS
            .iter()
            .map(|p| (*p, Vec::new()))
            .collect();

        // push all values supplied by args into their respective Placeholder's Vec
        for v in self.values.iter().copied() {
            variant_map.get_mut(&v.key()).unwrap().push(v);
        }

        // cross product of all Placeholder keys
        let mut variants: Vec<Vec<Values>> = Vec::from(&[Vec::new()]);
        for (p, mut values) in variant_map.into_iter() {
            // if some Placeholders don't have any values -> all possible values
            if values.is_empty() {
                values = Values::VARIANTS
                    .iter()
                    .copied()
                    .filter(|v| v.key() == p)
                    .collect();
            }
            assert!(!values.is_empty());
            values.sort_by_key(|v| v.key() as u32);

            variants = values
                .into_iter()
                .flat_map(|add| {
                    variants
                        .iter()
                        .map(move |v| v.iter().copied().chain([add]).collect::<Vec<_>>())
                })
                .collect::<Vec<_>>();
        }

        debug!("Computed variants: {variants:?}");
        variants
    }

    fn out_base_dir(&self) -> anyhow::Result<PathBuf> {
        let out = self
            .out
            .clone()
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../generated"));
        std::fs::create_dir_all(&out)?;
        Ok(out)
    }

    fn generate(&self, out_base_dir: &Path, variant: &[Values]) -> anyhow::Result<PathBuf> {
        let out_dir = {
            let mut out_dir = PathBuf::from(out_base_dir);
            for value in variant {
                out_dir.push(value.value());
            }
            std::fs::create_dir_all(&out_dir)?;
            out_dir
        };

        debug!("Generating `{variant:?}` at `{}`", out_dir.display());
        let mut args = GenerateArgs::default();
        args.template_path.path = Some(format!("{}/../ash-graphics", env!("CARGO_MANIFEST_DIR")));
        args.init = true;
        args.overwrite = false;
        args.define = variant
            .iter()
            .map(|v| format!("{}={}", v.key(), v.value()))
            .collect();
        args.name = Some("name-is-ignored".to_string());
        args.destination = Some(out_dir.clone());
        cargo_generate::generate(args)?;

        Ok(out_dir)
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let out_base_dir = self.out_base_dir()?;
        let variants = self.variants();
        let _results = variants
            .into_iter()
            .map(|variant| {
                let out_dir = self.generate(&out_base_dir, &variant)?;
                Ok((variant, out_dir))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn placeholder_map() -> HashMap<Placeholders, Vec<Values>> {
        let mut variant_map: HashMap<Placeholders, Vec<Values>> = Placeholders::VARIANTS
            .iter()
            .copied()
            .map(|p| (p, Vec::new()))
            .collect();
        for v in Values::VARIANTS.iter().copied() {
            variant_map.get_mut(&v.key()).unwrap().push(v);
        }
        debug!("placeholder_map: {variant_map:?}");
        variant_map
    }

    #[test]
    pub fn test_variants_all() {
        let v_all = Generate {
            values: Vec::new(),
            ..Default::default()
        }
        .variants();
        for (p, values) in placeholder_map() {
            debug!("{p}: {values:?}");
            assert_eq!(v_all.len() % values.len(), 0);
            let each_type_count = v_all.len() / values.len();

            for value in values {
                let vec = v_all
                    .iter()
                    .filter(|v| v.contains(&value))
                    .collect::<Vec<_>>();
                assert_eq!(
                    vec.len(),
                    each_type_count,
                    "Expected {each_type_count} variants with `{value}`, got {}: {vec:?}",
                    vec.len()
                );
            }
        }
    }

    #[test]
    pub fn test_variants_no_duplicates() {
        let generate = Generate {
            values: Vec::new(),
            ..Default::default()
        };
        let variants = generate.variants();
        for variant in variants {
            for value in variant.iter().copied() {
                let value_count = variant.iter().copied().filter(|o| *o == value).count();
                assert_eq!(
                    value_count, 1,
                    "Variant `{variant:?}` contains value `{value}` more than once!"
                )
            }
        }
    }

    #[test]
    pub fn test_variants_filter() {
        let v_all = Generate {
            values: Vec::new(),
            ..Default::default()
        }
        .variants();
        for (p, values) in placeholder_map() {
            debug!("{p}: {values:?}");
            assert_eq!(v_all.len() % values.len(), 0);
            let each_type_count = v_all.len() / values.len();

            for i in 1..values.len() {
                let v_one = Generate {
                    values: Vec::from(&values[..i]),
                    ..Default::default()
                }
                .variants();
                assert_eq!(v_one.len(), each_type_count * i);
            }
        }
    }
}
