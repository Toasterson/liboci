use std::fs::File;

use anyhow::Result;
use serde::de::DeserializeOwned;

use liboci::{Config, ContentDiscoveryResponse, ImageRef, OCIDir};

#[allow(dead_code)]
fn load_from_dockerhub(image_ref: &ImageRef) -> Result<ContentDiscoveryResponse> {
    let mut content_discovery_url = image_ref.get_v2_url()?;
    let base_path = content_discovery_url.path().to_owned();
    content_discovery_url.set_path((base_path + "/tags/list").as_str());
    let resp = reqwest::blocking::get(content_discovery_url)?;
    let resp = resp.json()?;
    Ok(resp)
}

fn load_from_samples<T: DeserializeOwned>(name: &str) -> Result<T> {
    let f = File::open(format!("samples/{name}.json"))?;
    Ok(serde_json::from_reader(f)?)
}

#[test]
fn test_get_ubuntu_config() -> Result<()> {
    let config: Config = load_from_samples("library-ubuntu-config")?;
    assert_eq!(config.architecture, "amd64");
    assert_eq!(config.os, "linux");
    Ok(())
}

#[test]
fn load_oci_dir() -> Result<()> {
    let dir = OCIDir::open("samples/ubuntu")?;
    assert_eq!(dir.index.schema_version, 2);
    assert_eq!(dir.manifests[0].schema_version, 2);
    Ok(())
}

#[test]
fn load_oci_dir2() -> Result<()> {
    let dir = OCIDir::open("samples/bitnami-postgresql")?;
    assert_eq!(dir.index.schema_version, 2);
    assert_eq!(dir.manifests[0].schema_version, 2);
    Ok(())
}

#[test]
fn read_write_oci_dir2() -> Result<()> {
    let dir = OCIDir::open("samples/bitnami-postgresql")?;
    assert_eq!(dir.index.schema_version, 2);
    assert_eq!(dir.manifests[0].schema_version, 2);
    let serialized = serde_json::to_value(&dir.configs[0])?;
    let mut f = File::open("samples/bitnami-postgresql/blobs/sha256/23e3fb27f226bdd839905bdf48698f1dc5f1b848962e85b74e8a2dec06dc19e0")?;
    let orig_value: serde_json::Value = serde_json::from_reader(&mut f)?;
    similar_asserts::assert_eq!(orig_value, serialized);
    Ok(())
}