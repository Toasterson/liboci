use std::fs::File;
use std::io::Write;
use anyhow::Result;
use schemars::schema::RootSchema;
use schemars::schema_for;
use liboci::{Config, ImageIndex, ImageLayout, ImageManifest};

fn main() -> Result<()> {
    let schemas = vec![
        (schema_for!(ImageIndex), "image-index"),
        (schema_for!(ImageLayout), "image-layout"),
        (schema_for!(ImageManifest), "image-manifest"),
        (schema_for!(Config), "config"),
    ];
    for schema in schemas {
        write_schema(schema.0, schema.1)?;
    }
    Ok(())
}

fn write_schema(root_schema: RootSchema, name: &str) -> Result<()> {
    let serialized = serde_json::to_string_pretty(&root_schema)?;
    let mut f = File::create(format!("target/{name}.json"))?;
    f.write_all(serialized.as_bytes())?;
    Ok(())
}