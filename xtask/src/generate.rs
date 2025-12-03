use crate::cargo_generate_config::{CONFIG_FILE_NAME, Config};
use anyhow::{Context, anyhow, bail};
use cargo_generate::GenerateArgs;
use clap::Parser;
use indexmap::IndexMap;
use log::{debug, info};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};

pub const TEMPLATE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../graphics");

#[derive(Parser, Debug, Default)]
pub struct Generate {
    /// Directory where to place the generated templates.
    #[clap(long)]
    out: Option<PathBuf>,
    /// Clean generate, deletes the generated directory before generating templates. Use this when you moved or removed
    /// files.
    #[clap(long)]
    clean: bool,
    /// A command that should be executed on each generated template.
    ///
    /// If a command fails, this process will fail as well, allowing you to test the template output.
    #[clap(long, short = 'x')]
    execute: Option<String>,
    /// Filter for values that any placeholder accepts
    ///
    /// We assume there are no values that two different placeholders match, within a single template, so we don't have
    /// to specify the placeholder the value is associated to.
    filter: Vec<String>,
}

#[derive(Clone, Debug)]
struct Template {
    name: String,
    template_dir: PathBuf,
    placeholders: IndexMap<String, Vec<String>>,
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct Define<'a> {
    key: &'a str,
    value: &'a str,
}

impl Display for Define<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.key, self.value)
    }
}

impl Debug for Define<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Template {
    fn graphics() -> anyhow::Result<Self> {
        Self::parse("graphics".to_string(), PathBuf::from(TEMPLATE_PATH))
    }

    fn parse(name: String, template_dir: PathBuf) -> anyhow::Result<Self> {
        let config_file = template_dir.join(CONFIG_FILE_NAME);
        let config: Config = toml::from_str(&std::fs::read_to_string(&config_file)?)?;
        let placeholders = config
            .placeholders
            .with_context(|| format!("Expected `placeholders` in `{}`", config_file.display()))?
            .0;
        let placeholders = placeholders
            .into_iter()
            .map(|(p, toml)| {
                let choices = toml.get("choices").with_context(|| {
                    format!(
                        "Expected `placeholders.{p}` in `{}` to have `choices` set",
                        config_file.display()
                    )
                })?;
                let choices = choices.as_array().with_context(|| {
                    format!(
                        "Expected `placeholders.{p}.choices` in `{}` to be an array",
                        config_file.display()
                    )
                })?;
                let choices =
                    choices
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            let c = c.as_str().with_context(|| format!(
                                "Expected `placeholders.{p}.choices[{i}]` in `{}` to be a string",
                                config_file.display()
                            ))?;
                            Ok(c.to_string())
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?;
                Ok((p, choices))
            })
            .collect::<anyhow::Result<IndexMap<_, _>>>()?;
        Ok(Self {
            name,
            template_dir,
            placeholders,
        })
    }

    fn value_to_placeholder(&self) -> HashMap<&str, &str> {
        self.placeholders
            .iter()
            .flat_map(|(key, values)| values.iter().map(|v| (v.as_str(), key.as_str())))
            .collect()
    }

    /// Computes all template variants to expand to and returns them in a Vec.
    /// The inner vec is guaranteed to contain no duplicate [`Placeholders`] and is sorted by order of the
    /// [`Placeholders`] in the enum. Returns an error if a filter was not found.
    fn variants<'a>(
        &'a self,
        filter: impl Iterator<Item = &'a str>,
    ) -> Result<Vec<Vec<Define<'a>>>, &'a str> {
        // insert all Placeholders with empty Vec for possible values
        let mut variant_map: IndexMap<&str, Vec<&str>> = self
            .placeholders
            .iter()
            .map(|(p, _)| (p.as_str(), Vec::new()))
            .collect();

        // push all values supplied by args into their respective Placeholder's Vec
        let mut filter = filter.peekable();
        if filter.peek().is_some() {
            let value_to_key = self.value_to_placeholder();
            for v in filter {
                let key = value_to_key.get(v).ok_or(v)?;
                variant_map.get_mut(key).unwrap().push(v);
            }
        }

        // cross product of all Placeholder keys
        let mut variants: Vec<Vec<Define>> = Vec::from(&[Vec::new()]);
        for (p, mut values) in variant_map.into_iter() {
            // if some Placeholders don't have any values -> all possible values
            if values.is_empty() {
                let all = self.placeholders.get(p).unwrap();
                values = all.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            }
            assert!(!values.is_empty());

            variants = values
                .into_iter()
                .flat_map(|add| {
                    variants.iter().map(move |v| {
                        v.iter()
                            .copied()
                            .chain([Define { key: p, value: add }])
                            .collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>();
        }

        debug!("Variants for template `{}`: {variants:?}", self.name);
        Ok(variants)
    }
}

impl Generate {
    fn out_base_dir(&self) -> anyhow::Result<PathBuf> {
        let out = self
            .out
            .clone()
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../generated"));
        debug!("out_base_dir: {}", out.display());
        if self.clean {
            std::fs::remove_dir_all(&out)?;
        }
        std::fs::create_dir_all(&out)?;
        Ok(out)
    }

    /// Some params can't be normalized with `--define`
    /// https://github.com/cargo-generate/cargo-generate/issues/1602
    fn normalize_env(&self) {
        // Safety: xtask generate is not multithreaded
        unsafe {
            std::env::set_var("CARGO_NAME", "generated");
            std::env::set_var("CARGO_EMAIL", "generated");
        }
    }

    fn generate(
        &self,
        out_base_dir: &Path,
        template: &Template,
        variant: &[Define],
    ) -> anyhow::Result<PathBuf> {
        let out_dir = {
            let mut out_dir = PathBuf::from(out_base_dir);
            for value in variant {
                out_dir.push(value.value);
            }
            std::fs::create_dir_all(&out_dir)?;
            out_dir
        };

        debug!("Generating `{variant:?}` at `{}`", out_dir.display());
        let mut args = GenerateArgs::default();
        args.template_path.path = Some(template.template_dir.to_string_lossy().into_owned());
        args.init = true;
        args.overwrite = true;
        args.silent = true;
        args.define = variant.iter().map(|v| v.to_string()).collect();
        args.name = Some("name-is-ignored".to_string());
        args.destination = Some(out_dir.clone());
        cargo_generate::generate(args)?;

        Ok(out_dir)
    }

    fn execute<'a>(&self, out_dirs: impl Iterator<Item = &'a Path>) -> anyhow::Result<()> {
        if let Some(execute) = &self.execute {
            let mut split = execute.split(" ");
            // split iterator has at least one entry
            let exec = split.next().unwrap();
            let args = split.collect::<Vec<_>>();

            let mut success = true;
            for out_dir in out_dirs {
                let mut cmd = std::process::Command::new(exec);
                cmd.args(args.iter()).current_dir(out_dir);
                info!("Spawning process: {cmd:?}");
                let status = cmd.spawn()?.wait().context("Process spawning failed")?;
                success &= status.success();
            }
            if !success {
                bail!("Some processes spawned by `--execute` failed");
            }
        }
        Ok(())
    }

    pub fn run(&self) -> anyhow::Result<()> {
        self.normalize_env();
        let out_base_dir = self.out_base_dir()?;

        let template = Template::graphics()?;
        let variants = template
            .variants(self.filter.iter().map(|f| f.as_str()))
            .map_err(|filter| anyhow!("Unknown filter `{filter}`"))?;
        let results = variants
            .iter()
            .map(|variant| self.generate(&out_base_dir, &template, variant))
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.execute(results.iter().map(|a| a.as_path()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARGO_GPU: Define = Define {
        key: "integration",
        value: "cargo-gpu",
    };
    const SPIRV_BUILDER: Define = Define {
        key: "integration",
        value: "spirv-builder",
    };
    const ASH: Define = Define {
        key: "api",
        value: "ash",
    };
    const WGPU: Define = Define {
        key: "api",
        value: "wgpu",
    };
    const CPU: Define = Define {
        key: "api",
        value: "cpu",
    };

    pub fn test_template() -> Template {
        Template {
            name: "my-template".to_string(),
            template_dir: PathBuf::from("./my_template/"),
            placeholders: IndexMap::from(
                [
                    ("integration", ["cargo-gpu", "spirv-builder"].as_slice()),
                    ("api", ["ash", "wgpu", "cpu"].as_slice()),
                ]
                .map(|(k, v)| {
                    (
                        k.to_string(),
                        v.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
                    )
                }),
            ),
        }
    }

    #[test]
    pub fn variants_all() {
        let template = test_template();
        let all = template.variants(std::iter::empty()).unwrap();
        debug!("all: {all:?}");

        // order *in the outer slice* is arbitrary
        // order within the inner slice is not, and will affect `generated/` folder structure
        let expected = [
            [CARGO_GPU, ASH],
            [SPIRV_BUILDER, ASH],
            [CARGO_GPU, WGPU],
            [SPIRV_BUILDER, WGPU],
            [CARGO_GPU, CPU],
            [SPIRV_BUILDER, CPU],
        ];
        assert_eq!(all, expected);
    }

    #[test]
    pub fn variants_cross_product_test() {
        variants_cross_product(test_template());
    }

    #[test]
    pub fn variants_cross_product_repo() {
        variants_cross_product(Template::graphics().unwrap());
    }

    pub fn variants_cross_product(template: Template) {
        let variants = template.variants(std::iter::empty()).unwrap();
        for variant in variants {
            for value in variant.iter() {
                let value_count = variant.iter().filter(|o| Define::eq(o, value)).count();
                assert_eq!(
                    value_count, 1,
                    "Variant `{variant:?}` contains value `{value}` more than once!"
                )
            }
        }
    }

    #[test]
    pub fn variants_filter_test() {
        variants_filter(test_template())
    }

    #[test]
    pub fn variants_filter_repo() {
        variants_filter(Template::graphics().unwrap())
    }

    pub fn variants_filter(template: Template) {
        let v_all = template.variants(std::iter::empty()).unwrap();
        for (p, values) in &template.placeholders {
            debug!("{p}: {values:?}");
            assert_eq!(v_all.len() % values.len(), 0);
            let each_type_count = v_all.len() / values.len();

            for i in 1..values.len() {
                let v_one = template
                    .variants(values[..i].iter().map(|s| s.as_str()))
                    .unwrap();
                assert_eq!(v_one.len(), each_type_count * i);
            }
        }
    }
}
