use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::Path;
use indexmap::IndexMap;
use schemars::{JsonSchema};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use base64::{Engine as _, engine::{general_purpose}};
use chrono::Utc;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use thiserror::Error;
use url::Url;

fn base64_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
    let mut obj = SchemaObject::default();
    obj.extensions.insert("media".to_owned(), json!({"binaryEncoding": "base64"}));
    Schema::Object(obj)
}

#[derive(Debug)]
/// OCI dir layout struct
pub struct OCIDir {
    pub index: ImageIndex,
    pub manifests: IndexMap<String, ImageManifest>,
    pub configs: IndexMap<String, Config>,
}

fn load_file_helper<T: DeserializeOwned>(path: &Path) -> Result<T, OCIDirError> {
    let f = File::open(path)?;
    Ok(serde_json::from_reader(f)?)
}

impl OCIDir {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, OCIDirError> {
        let index: ImageIndex = load_file_helper(&path.as_ref().join("index.json"))?;
        let mut manifests = IndexMap::new();
        let mut configs = IndexMap::new();

        for manifest_descr in index.manifests.iter() {
            let image_manifest: ImageManifest = load_file_helper(&path.as_ref().join("blobs").join(manifest_descr.digest.replace(":", "/")))?;
            let config_digest = image_manifest.config.digest.clone();
            let config_manifest: Config = if let Some(data) = &image_manifest.config.data {
                serde_json::from_slice(data.0.as_slice())?
            } else {
                load_file_helper(&path.as_ref().join("blobs").join(config_digest.replace(":", "/")))?
            };
            manifests.insert(manifest_descr.digest.clone(), image_manifest);
            configs.insert(config_digest, config_manifest);
        }

        Ok( Self {
            index,
            manifests,
            configs,
        })
    }

}

#[derive(Debug, Error)]
pub enum OCIDirError {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum ImageRefError {
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentDiscoveryResponse {
    pub name: String,
    pub tags: Vec<String>,
}

pub struct ImageRef {
    use_ssl: bool,
    host: String,
    name: String,
}

impl Display for ImageRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.host, self.name)
    }
}

impl ImageRef {
    pub fn new<S: ToString>(host: S, name: S, use_ssl: bool) -> Self {
        Self {
            use_ssl,
            host: host.to_string(),
            name: name.to_string(),
        }
    }

    pub fn get_v2_url(&self) -> Result<Url, ImageRefError> {
        let schema = if self.use_ssl {
            "https"
        } else {
            "http"
        };
        Ok(format!("{schema}://{}/v2/{}", self.host, self.name).parse()?)
    }
}

#[derive(JsonSchema, Debug)]
pub struct Base64(Vec<u8>);
impl Serialize for Base64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(general_purpose::STANDARD.encode(&self.0).as_str())
    }
}

impl<'de> Deserialize<'de> for Base64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Vis;
        impl serde::de::Visitor<'_> for Vis {
            type Value = Base64;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a base64 string")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                general_purpose::STANDARD.decode(v).map(Base64).map_err(Error::custom)
            }
        }
        deserializer.deserialize_str(Vis)
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
/// OpenContainer Content Descriptor Specification
pub struct ContentDescriptor {
    /// the mediatype of the referenced object
    #[validate(regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}/[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}$"))]
    pub media_type: String,
    /// the size in bytes of the referenced object
    #[validate(range(min = -9223372036854775808i64, max = 9223372036854775807i64))]
    #[schemars(with = "i64")]
    pub size: i64,
    /// the cryptographic checksum digest of the object, in the pattern '<algorithm>:<encoded>'
    #[schemars(regex(pattern = r"^[a-z0-9]+(?:[+._-][a-z0-9]+)*:[a-zA-Z0-9=_-]+$"))]
    pub digest: String,
    /// a list of urls from which this object may be downloaded
    #[schemars(inner(url))]
    pub urls: Option<Vec<Url>>,
    /// an embedding of the targeted content (base64 encoded)
    #[schemars(schema_with = "base64_schema", default)]
    pub data: Option<Base64>,
    /// the IANA media type of this artifact
    #[validate(regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}/[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}$"))]
    pub artifact_type: Option<String>,
    #[validate(inner(regex(pattern = r".{1,}")))]
    #[serde(default)]
    #[schemars(schema_with = "annotation_schema")]
    pub annotations: IndexMap<String, String>,
}

fn annotation_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    let mut obj = SchemaObject::default();
    obj.instance_type = Some(SingleOrVec::Single(Box::new(InstanceType::Object)));
    let mut str_obj = SchemaObject::default();
    str_obj.instance_type = Some(SingleOrVec::Single(Box::new(InstanceType::String)));
    obj.object().pattern_properties.insert(".{1,}".to_owned(), Schema::Object(str_obj));
    schemars::schema::Schema::Object(obj)
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
/// OpenContainer Image Index Specification
pub struct ImageIndex {
    /// This field specifies the image index schema version as an integer
    #[validate(range(min = 2, max = 2))]
    pub schema_version: u8,
    /// the mediatype of the referenced object
    #[validate(regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}/[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}$"))]
    pub media_type: Option<String>,
    /// the artifact mediatype of the referenced object
    #[validate(regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}/[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}$"))]
    pub artifact_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<ContentDescriptor>,

    pub manifests: Vec<Manifest>,
    #[validate(inner(regex(pattern = r".{1,}")))]
    #[serde(default)]
    #[schemars(schema_with = "annotation_schema")]
    pub annotations: IndexMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// the mediatype of the referenced object
    #[validate(regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}/[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}$"))]
    pub media_type: String,
    /// the size in bytes of the referenced object
    #[validate(range(min = -9223372036854775808i64, max = 9223372036854775807i64))]
    #[schemars(with = "i64")]
    pub size: i64,
    /// the cryptographic checksum digest of the object, in the pattern '<algorithm>:<encoded>'
    #[schemars(regex(pattern = r"^[a-z0-9]+(?:[+._-][a-z0-9]+)*:[a-zA-Z0-9=_-]+$"))]
    pub digest: String,
    /// a list of urls from which this object may be downloaded
    #[schemars(inner(url))]
    pub urls: Option<Vec<Url>>,

    pub platform: Option<Platform>,

    #[validate(inner(regex(pattern = r".{1,}")))]
    #[serde(default)]
    #[schemars(schema_with = "annotation_schema")]
    pub annotations: IndexMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub architecture: String,
    pub os: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "os.version", default)]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", rename = "os.features", default)]
    pub os_features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub variant: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImageLayoutVersion {
    #[serde(rename = "1.0.0")]
    OneZeroZero
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
/// OpenContainer Image Layout Schema
pub struct ImageLayout {
    /// version of the OCI Image Layout (in the oci-layout file)
    image_layout_version: ImageLayoutVersion
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
/// OpenContainer Image Manifest Specification
pub struct ImageManifest {
    /// This field specifies the image index schema version as an integer
    #[validate(range(min = 2, max = 2))]
    pub schema_version: u8,
    /// the mediatype of the referenced object
    #[validate(regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}/[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}$"))]
    pub media_type: Option<String>,
    /// the artifact mediatype of the referenced object
    #[validate(regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}/[A-Za-z0-9][A-Za-z0-9!#$&-^_.+]{0,126}$"))]
    pub artifact_type: Option<String>,
    pub config: ContentDescriptor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<ContentDescriptor>,
    #[validate(length(min = 1))]
    pub layers: Vec<ContentDescriptor>,
    #[validate(inner(regex(pattern = r".{1,}")))]
    #[serde(default)]
    #[schemars(schema_with = "annotation_schema")]
    pub annotations: IndexMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
/// OpenContainer Config Specification
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created: Option<chrono::DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub author: Option<String>,

    pub architecture: String,
    pub os: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "os.version", default)]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", rename = "os.features", default)]
    pub os_features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<AppConfig>,
    pub rootfs: RootFS,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<HistoryEntry>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct AppConfig {
    pub user: Option<String>,
    #[validate(inner(regex(pattern = r".{1,}")))]
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub exposed_ports: IndexMap<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<String>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub volumes: Option<IndexMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub working_dir: Option<String>,
    pub labels: Option<IndexMap<String,String>>,
    pub stop_signal: Option<String>,
    #[serde(default)]
    pub args_escaped: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RootFSKind {
    Layers,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RootFS {
    #[serde(rename = "type")]
    pub kind: RootFSKind,
    pub diff_ids: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct HistoryEntry {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created: Option<chrono::DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub empty_layer: bool,
}