use crate::FlError;
use serde::Deserialize;

#[derive(Debug)]
pub struct PubVersions {
    pub name: String,
    pub latest: String,
    pub versions: Vec<String>,
}

#[derive(Deserialize)]
struct PubDevPackage {
    latest: PubDevVersion,
    versions: Vec<PubDevVersion>,
}

#[derive(Deserialize)]
struct PubDevVersion {
    version: String,
}

pub async fn fetch_dep_versions(package_name: &str) -> Result<PubVersions, FlError> {
    let url = format!("https://pub.dev/api/packages/{}", package_name);
    let res = reqwest::get(url).await?.json::<PubDevPackage>().await?;

    Ok(PubVersions {
        name: package_name.to_owned(),
        latest: res.latest.version,
        versions: res.versions.into_iter().map(|v| v.version).collect(),
    })
}
