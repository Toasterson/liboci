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
    println!("{:#?}", config);
    Ok(())
}

#[test]
fn load_oci_dir() -> Result<()> {
    let dir = OCIDir::open("samples/ubuntu")?;
    println!("{:#?}", dir);
    assert_eq!(dir.index.schema_version, 2);
    assert_eq!(dir.manifests[0].schema_version, 2);
    Ok(())
}