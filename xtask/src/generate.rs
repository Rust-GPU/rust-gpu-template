use crate::cargo_generate_config::{CONFIG_FILE_NAME, Config};
use anyhow::{Context, bail};
use cargo_generate::GenerateArgs;
use clap::Parser;
use indexmap::IndexMap;
use log::{debug, info};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};

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
struct TemplateDiscovery {
    templates: Vec<Template>,
}

impl TemplateDiscovery {
    pub const TEMPLATE_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

    fn discover() -> anyhow::Result<Self> {
        Self::discover_at(Path::new(Self::TEMPLATE_PATH))
    }

    /// Our discovery is not as dynamic as `cargo_generate`, written to be just sufficient for this repo and to be
    /// *fast*, even with a large target directory. Primarily, that means not scanning through the entire dir tree,
    /// instead just using the paths that are explicitly defined in the config.
    /// https://github.com/cargo-generate/cargo-generate/issues/1600
    fn discover_at(base_dir: &Path) -> anyhow::Result<Self> {
        let sub_templates = {
            let root_file = base_dir.join(CONFIG_FILE_NAME);
            let root: Config = toml::from_str(&std::fs::read_to_string(&root_file)?)?;
            let root_template = root
                .template
                .with_context(|| format!("Expected `template` in `{}`", root_file.display()))?;
            root_template.sub_templates.unwrap_or_else(Vec::new)
        };

        let templates = sub_templates
            .into_iter()
            .map(|name| {
                let template_dir = base_dir.join(&name);
                Template::parse(name, template_dir)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let discovery = Self { templates };
        debug!("Discovery found: {discovery:#?}");
        Ok(discovery)
    }

    fn split_filter<'a>(&self, filters: impl Iterator<Item = &'a str>) -> Filters<'a> {
        let mut out = Filters::default();
        for filter in filters {
            if self.templates.iter().any(|t| t.name == filter) {
                out.template_filters.insert(filter);
            } else {
                out.placeholder_filters.push(filter);
            }
        }
        debug!("Filters: {out:?}");
        out
    }

    fn filter_variants<'a>(
        &'a self,
        filters: impl Iterator<Item = &'a str>,
    ) -> anyhow::Result<Vec<(&'a Template, Vec<Define<'a>>)>> {
        let filters = self.split_filter(filters);

        let mut has_unknown_filter = true;
        let mut unknown_filter = None;
        let variants = self
            .templates
            .iter()
            .filter(|template| {
                filters.template_filters.is_empty()
                    || filters.template_filters.contains(template.name.as_str())
            })
            .flat_map(|template| {
                let variants_result =
                    template.variants(filters.placeholder_filters.iter().copied());
                let variants = match variants_result {
                    Ok(e) => {
                        has_unknown_filter = false;
                        e
                    }
                    Err(e) => {
                        unknown_filter = Some(e);
                        Vec::new()
                    }
                };
                variants.into_iter().map(move |v| (template, v))
            })
            .collect::<Vec<_>>();
        if has_unknown_filter {
            if let Some(filter) = unknown_filter {
                bail!("Unknown filter `{filter}`")
            } else {
                // Only reachable if no templates exist, Or if all templates have been filtered out, but you must filter
                // for at least one template for template filtering to even activate, so should be unreachable.
                bail!("No templates exist?")
            }
        }
        debug!("Variants: {variants:?}");
        Ok(variants)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Filters<'a> {
    template_filters: HashSet<&'a str>,
    /// value whether it was used
    placeholder_filters: Vec<&'a str>,
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

        let discovery = TemplateDiscovery::discover()?;
        let variants = discovery.filter_variants(self.filter.iter().map(|a| a.as_str()))?;
        let results = variants
            .iter()
            .map(|(template, variants)| {
                self.generate(&out_base_dir.join(&template.name), template, variants)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        if results.is_empty() {
            // reachable with two templates with differing placeholders and filtering for both
            bail!("Nothing generated, all variants filtered out");
        }

        self.execute(results.iter().map(|b| b.as_path()))?;
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
        variants_cross_product(&test_template());
    }

    #[test]
    pub fn variants_cross_product_repo() {
        let discovery = TemplateDiscovery::discover().unwrap();
        for template in &discovery.templates {
            debug!("Template {}:", template.name);
            variants_cross_product(template);
        }
    }

    pub fn variants_cross_product(template: &Template) {
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
        variants_filter(&test_template())
    }

    #[test]
    pub fn variants_filter_repo() {
        let discovery = TemplateDiscovery::discover().unwrap();
        for template in &discovery.templates {
            debug!("Template {}:", template.name);
            variants_filter(template);
        }
    }

    pub fn variants_filter(template: &Template) {
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

    #[test]
    pub fn template_filter_all_test() {
        let discovery = TemplateDiscovery {
            templates: Vec::from([test_template()]),
        };
        let filter = discovery.split_filter(std::iter::empty());
        assert_eq!(filter, Filters::default());
        let result = discovery.filter_variants(std::iter::empty()).unwrap();
        let templates = result
            .iter()
            .map(|(t, _)| t.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(templates, ["my-template"; 6])
    }

    #[test]
    pub fn template_filter_one_test() {
        let discovery = TemplateDiscovery {
            templates: Vec::from([test_template()]),
        };
        let filter_args = ["my-template"];
        let filter = discovery.split_filter(filter_args.iter().copied());
        assert_eq!(
            filter,
            Filters {
                template_filters: HashSet::from(["my-template"]),
                placeholder_filters: Vec::new(),
            }
        );
        let result = discovery
            .filter_variants(filter_args.iter().copied())
            .unwrap();
        let templates = result
            .iter()
            .map(|(t, _)| t.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(templates, ["my-template"; 6])
    }

    #[test]
    pub fn template_filter_none_test() {
        let discovery = TemplateDiscovery {
            templates: Vec::from([test_template()]),
        };
        let filter_args = ["unknown"];
        let filter = discovery.split_filter(filter_args.iter().copied());
        assert_eq!(
            filter,
            Filters {
                template_filters: HashSet::default(),
                placeholder_filters: Vec::from(["unknown"]),
            }
        );
        let result = discovery.filter_variants(filter_args.iter().copied());
        // Unknown filter
        assert!(result.is_err(), "Result: {result:#?}");
    }
}
