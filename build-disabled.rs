use std::fs::File;
use std::io::Write;
use schemars::schema::{RootSchema, Schema};
use typify::TypeSpaceSettings;

fn main() -> anyhow::Result<()> {
    let mut settings = typify::TypeSpaceSettings::default();
    settings.with_unknown_crates(typify::UnknownPolicy::Allow)
        .with_crate("liboci", typify::CrateVers::Version("0.1.0".parse()?), None);
    settings.with_struct_builder(false);
    for schema in vec!["defs", "config-schema", "image-index", "content-descriptor"] {
        create_code_from_schema(schema, &settings)?;
    }
    Ok(())
}

fn create_code_from_schema(name: &str, settings: &TypeSpaceSettings) -> anyhow::Result<()> {
    let config_schema = load_root_schema(name)?;
    let mut  space = typify::TypeSpace::new(settings);
    space.add_type(&load_schema("defs")?)?;
    space.add_root_schema(config_schema)?;
    let formated_schema = rustfmt_wrapper::rustfmt(space.to_stream().to_string())?;
    write_code(name, formated_schema)?;
    Ok(())
}

fn write_code(name: &str, code: String) -> anyhow::Result<()> {
    let name = name.replace("-", "_");
    let mut f = File::create(format!("src/{name}.rs"))?;
    f.write_all(code.as_bytes())?;
    Ok(())
}

fn load_root_schema(name: &str) -> anyhow::Result<RootSchema> {
    let f = File::open(format!("schema/{name}.json"))?;
    let schema = serde_json::from_reader(&f)?;
    Ok(schema)
}

fn load_schema(name: &str) -> anyhow::Result<Schema> {
    let f = File::open(format!("schema/{name}.json"))?;
    let schema = serde_json::from_reader(&f)?;
    Ok(schema)
}