use crate::{error::SvmError, platform::Platform};
use once_cell::sync::Lazy;
use reqwest::get;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use url::Url;


const ZKSOLC_RELEASES_URL: &str = "https://github.com/dutterbutter/zksolc-bin/tree/db/generate-list";

// Update URL prefixes for the specific platforms where binaries are stored
static LINUX_AARCH64_URL_PREFIX: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/linux-arm64";
static LINUX_AARCH64_RELEASES_URL: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/linux-arm64/list.json";

static MACOS_AARCH64_URL_PREFIX: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/macosx-arm64";
static MACOS_AARCH64_RELEASES_URL: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/macosx-arm64/list.json";

static MACOS_AMD64_URL_PREFIX: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/macosx-amd64";
static MACOS_AMD64_RELEASES_URL: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/macosx-amd64/list.json";

static LINUX_AMD64_URL_PREFIX: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/linux-amd64";
static LINUX_AMD64_RELEASES_URL: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/linux-amd64/list.json";

static WINDOWS_AMD64_URL_PREFIX: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/windows-amd64";
static WINDOWS_AMD64_RELEASES_URL: &str = "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/windows-amd64/list.json";

const VERSION_MAX: Version = Version::new(1, 4, 1);
const VERSION_MIN: Version = Version::new(1, 3, 13);


/// Defines the struct that the JSON-formatted release list can be deserialized into.
///
/// Both the key and value are deserialized into [`semver::Version`].
///
/// ```json
/// {
///     "builds": [
///         {
///             "version": "0.8.7",
///             "sha256": "0x0xcc5c663d1fe17d4eb4aca09253787ac86b8785235fca71d9200569e662677990"
///         }
///     ]
///     "releases": {
///         "0.8.7": "solc-macosx-amd64-v0.8.7+commit.e28d00a7",
///         "0.8.6": "solc-macosx-amd64-v0.8.6+commit.11564f7e",
///         ...
///     }
/// }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Releases {
    pub builds: Vec<BuildInfo>,
    pub releases: BTreeMap<Version, String>,
}

impl Releases {
    /// Get the checksum of a solc version's binary if it exists.
    pub fn get_checksum(&self, v: &Version) -> Option<Vec<u8>> {
        for build in self.builds.iter() {
            if build.version.eq(v) {
                return Some(build.sha256.clone());
            }
        }
        None
    }

    /// Returns the artifact of the version if any
    pub fn get_artifact(&self, version: &Version) -> Option<&String> {
        self.releases.get(version)
    }

    /// Returns a sorted list of all versions
    pub fn into_versions(self) -> Vec<Version> {
        let mut versions = self.releases.into_keys().collect::<Vec<_>>();
        versions.sort_unstable();
        versions
    }
}

/// Build info contains the SHA256 checksum of a solc binary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: Version,
    #[serde(with = "hex_string")]
    pub sha256: Vec<u8>,
}

/// Helper serde module to serialize and deserialize bytes as hex.
mod hex_string {
    use super::*;
    use serde::{de, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        hex::decode(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: AsRef<[u8]>,
    {
        serializer.serialize_str(&hex::encode_prefixed(value))
    }
}

/// Blocking version of [`all_releases`].
#[cfg(feature = "blocking")]
pub fn blocking_all_releases(platform: Platform) -> Result<Releases, SvmError> {
    match platform {
        Platform::LinuxAarch64 => {
            Ok(reqwest::blocking::get(LINUX_AARCH64_RELEASES_URL)?.json::<Releases>()?)
        }
        Platform::MacOsAarch64 => {
            Ok(reqwest::blocking::get(MACOS_AARCH64_RELEASES_URL)?.json::<Releases>()?)         
        }
        Platform::MacOsAmd64 => {
            Ok(reqwest::blocking::get(MACOS_AMD64_RELEASES_URL)?.json::<Releases>()?)         
        }
        Platform::LinuxAmd64 => {
            Ok(reqwest::blocking::get(LINUX_AMD64_RELEASES_URL)?.json::<Releases>()?)         
        }
        Platform::WindowsAmd64 => {
            Ok(reqwest::blocking::get(WINDOWS_AMD64_RELEASES_URL)?.json::<Releases>()?)         
        }
        _ => {
            // TODO fix this
            let releases =
                reqwest::blocking::get(format!("{ZKSOLC_RELEASES_URL}/{platform}/list.json"))?
                    .json::<Releases>()?;
            Ok(unified_releases(releases, platform))
        }
    }
}

/// Fetch all releases available for the provided platform.
pub async fn all_releases(platform: Platform) -> Result<Releases, SvmError> {
    match platform {
        Platform::LinuxAarch64 => Ok(get(LINUX_AARCH64_RELEASES_URL)
            .await?
            .json::<Releases>()
            .await?),
        Platform::MacOsAarch64 => 
            Ok(get(MACOS_AARCH64_RELEASES_URL)
            .await?
            .json::<Releases>()
            .await?),
        Platform::MacOsAmd64 => 
            Ok(get(MACOS_AMD64_RELEASES_URL)
            .await?
            .json::<Releases>()
            .await?),
        Platform::LinuxAmd64 =>
            Ok(get(LINUX_AMD64_RELEASES_URL)
            .await?
            .json::<Releases>()
            .await?),
        Platform::WindowsAmd64 =>
            Ok(get(WINDOWS_AMD64_RELEASES_URL)
            .await?
            .json::<Releases>()
            .await?),
        _ => {
            // TODO fix this
            let releases = get(format!("{ZKSOLC_RELEASES_URL}/{platform}/list.json"))
            .await?
            .json::<Releases>()
            .await?;

        Ok(unified_releases(releases, platform))
        }
    }
}

/// unifies the releases with old releases if on linux
// TODO: remove this function once all platforms have been updated
fn unified_releases(releases: Releases, platform: Platform) -> Releases {
    releases
}

/// Construct the URL to the Solc binary for the specified release version and target platform.
pub(crate) fn artifact_url(
    platform: Platform,
    version: &Version,
    artifact: &str,
) -> Result<Url, SvmError> {
    if platform == Platform::LinuxAmd64 {
        if *version >= VERSION_MIN && *version <= VERSION_MAX {
            return Ok(Url::parse(&format!(
                "{LINUX_AMD64_URL_PREFIX}/{artifact}"
            ))?);
        } else {
            return Err(SvmError::UnsupportedVersion(
                version.to_string(),
                platform.to_string(),
            ));
        }
    }  

    if platform == Platform::LinuxAarch64 {
        if *version >= VERSION_MIN && *version <= VERSION_MAX {
            return Ok(Url::parse(&format!(
                "{LINUX_AARCH64_URL_PREFIX}/{artifact}"
            ))?);
        } else {
            return Err(SvmError::UnsupportedVersion(
                version.to_string(),
                platform.to_string(),
            ));
        }
    }

    if  *version < VERSION_MIN {
        return Err(SvmError::UnsupportedVersion(
            version.to_string(),
            platform.to_string(),
        ));
    }

    if platform == Platform::MacOsAarch64 {
        if *version >= VERSION_MIN && *version <= VERSION_MAX {
            // fetch natively build solc binaries from `https://github.com/alloy-rs/solc-builds`
            return Ok(Url::parse(&format!(
                "{MACOS_AARCH64_URL_PREFIX}/{artifact}"
            ))?);
        } else {
            return Err(SvmError::UnsupportedVersion(
                version.to_string(),
                platform.to_string(),
            ));
        }
    }
    if platform == Platform::MacOsAmd64 {
        if *version >= VERSION_MIN && *version <= VERSION_MAX {
            return Ok(Url::parse(&format!(
                "{MACOS_AMD64_URL_PREFIX}/{artifact}"
            ))?);
        } else {
            return Err(SvmError::UnsupportedVersion(
                version.to_string(),
                platform.to_string(),
            ));
        }
    }
    if platform == Platform::WindowsAmd64 {
        if *version >= VERSION_MIN && *version <= VERSION_MAX {
            return Ok(Url::parse(&format!(
                "{WINDOWS_AMD64_URL_PREFIX}/{artifact}"
            ))?);
        } else {
            return Err(SvmError::UnsupportedVersion(
                version.to_string(),
                platform.to_string(),
            ));
        }
    }

    Ok(Url::parse(&format!(
        "{ZKSOLC_RELEASES_URL}/{platform}/{artifact}"
    ))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_url() {
        let version = Version::new(1, 3, 17);
        let artifact = "zksolc-linux-arm64-musl-v1.3.17";
        assert_eq!(
            artifact_url(Platform::LinuxAarch64, &version, artifact).unwrap(),
            Url::parse(&format!(
                "https://github.com/dutterbutter/zksolc-bin/raw/db/generate-list/linux-arm64/{artifact}"
            ))
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn test_all_releases_macos_amd64() {
        assert!(all_releases(Platform::MacOsAmd64).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_releases_macos_aarch64() {
        assert!(all_releases(Platform::MacOsAarch64).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_releases_linux_amd64() {
        assert!(all_releases(Platform::LinuxAmd64).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_releases_linux_aarch64() {
        assert!(all_releases(Platform::LinuxAarch64).await.is_ok());
    }

    #[tokio::test]
    async fn releases_roundtrip() {
        let releases = all_releases(Platform::LinuxAmd64).await.unwrap();
        let s = serde_json::to_string(&releases).unwrap();
        let de_releases: Releases = serde_json::from_str(&s).unwrap();
        assert_eq!(releases, de_releases);
    }
}
