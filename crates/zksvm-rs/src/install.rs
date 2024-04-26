use crate::{
    all_releases, data_dir, platform, releases::artifact_url, setup_data_dir, setup_version,
    version_binary, SvmError,
};
use semver::Version;
use sha2::Digest;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::Duration,
};

#[cfg(target_family = "unix")]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

/// The timeout to use for requests to the source
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Blocking version of [`install`]
#[cfg(feature = "blocking")]
pub fn blocking_install(version: &Version) -> Result<PathBuf, SvmError> {
    setup_data_dir()?;

    let artifacts = crate::blocking_all_releases(platform::platform())?;
    let artifact = artifacts
        .get_artifact(version)
        .ok_or(SvmError::UnknownVersion)?;
    let download_url = artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    let expected_checksum = artifacts
        .get_checksum(version)
        .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    let res = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("reqwest::Client::new()")
        .get(download_url.clone())
        .send()?;

    if !res.status().is_success() {
        return Err(SvmError::UnsuccessfulResponse(download_url, res.status()));
    }

    let binbytes = res.bytes()?;
    ensure_checksum(&binbytes, version, &expected_checksum)?;

    // lock file to indicate that installation of this zksolc version will be in progress.
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of zksolc.
    let _lock = try_lock_file(lock_path)?;

    do_install(version, &binbytes, artifact.to_string().as_str())
}

/// Installs the provided version of zksolc in the machine.
///
/// Returns the path to the zksolc file.
pub async fn install(version: &Version) -> Result<PathBuf, SvmError> {
    setup_data_dir()?;

    let artifacts = all_releases(platform::platform()).await?;
    let artifact = artifacts
        .releases
        .get(version)
        .ok_or(SvmError::UnknownVersion)?;
    let download_url = artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    let expected_checksum = artifacts
        .get_checksum(version)
        .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    let res = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("reqwest::Client::new()")
        .get(download_url.clone())
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(SvmError::UnsuccessfulResponse(download_url, res.status()));
    }

    let binbytes = res.bytes().await?;
    ensure_checksum(&binbytes, version, &expected_checksum)?;

    // lock file to indicate that installation of this zksolc version will be in progress.
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of zksolc.
    let _lock = try_lock_file(lock_path)?;

    do_install(version, &binbytes, artifact.to_string().as_str())
}

fn do_install(version: &Version, binbytes: &[u8], _artifact: &str) -> Result<PathBuf, SvmError> {
    setup_version(&version.to_string())?;
    let installer = Installer { version, binbytes };

    // zksolc versions <= 0.7.1 are .zip files for Windows only
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    if _artifact.ends_with(".zip") {
        return installer.install_zip();
    }

    installer.install()
}

/// Creates the file and locks it exclusively, this will block if the file is currently locked
fn try_lock_file(lock_path: PathBuf) -> Result<LockFile, SvmError> {
    use fs4::FileExt;
    let _lock_file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    _lock_file.lock_exclusive()?;
    Ok(LockFile {
        lock_path,
        _lock_file,
    })
}

/// Represents a lockfile that's removed once dropped
struct LockFile {
    _lock_file: fs::File,
    lock_path: PathBuf,
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Returns the lockfile to use for a specific file
fn lock_file_path(version: &Version) -> PathBuf {
    data_dir().join(format!(".lock-zksolc-{version}"))
}

// Installer type that copies binary data to the appropriate zksolc binary file:
// 1. create target file to copy binary data
// 2. copy data
struct Installer<'a> {
    // version of zksolc
    version: &'a Version,
    // binary data of the zksolc executable
    binbytes: &'a [u8],
}

impl Installer<'_> {
    /// Installs the zksolc version at the version specific destination and returns the path to the installed zksolc file.
    fn install(self) -> Result<PathBuf, SvmError> {
        let zksolc_path = version_binary(&self.version.to_string());
        
        let mut f = fs::File::create(&zksolc_path)?;
        #[cfg(target_family = "unix")]
        f.set_permissions(Permissions::from_mode(0o755))?;
        f.write_all(self.binbytes)?;

        Ok(zksolc_path)
    }

    /// Extracts the zksolc archive at the version specified destination and returns the path to the
    /// installed zksolc binary.
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    fn install_zip(self) -> Result<PathBuf, SvmError> {
        let zksolc_path = version_binary(&self.version.to_string());
        let version_path = zksolc_path.parent().unwrap();

        let mut content = std::io::Cursor::new(self.binbytes);
        let mut archive = zip::ZipArchive::new(&mut content)?;
        archive.extract(version_path)?;

        std::fs::rename(version_path.join("zksolc.exe"), &zksolc_path)?;

        Ok(zksolc_path)
    }
}

fn ensure_checksum(
    binbytes: &[u8],
    version: &Version,
    expected_checksum: &[u8],
) -> Result<(), SvmError> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(binbytes);
    let checksum = &hasher.finalize()[..];
    // checksum does not match
    if checksum != expected_checksum {
        return Err(SvmError::ChecksumMismatch {
            version: version.to_string(),
            expected: hex::encode(expected_checksum),
            actual: hex::encode(checksum),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::seq::SliceRandom;

    #[allow(unused)]
    const LATEST: Version = Version::new(1, 4,1);

    #[tokio::test]
    #[serial_test::serial]
    async fn test_install() {
        let versions = all_releases(platform())
            .await
            .unwrap()
            .releases
            .into_keys()
            .collect::<Vec<Version>>();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(install(rand_version).await.is_ok());
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    fn blocking_test_install() {
        let versions = crate::releases::blocking_all_releases(platform::platform())
            .unwrap()
            .into_versions();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(blocking_install(rand_version).is_ok());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_version() {
        let version = "1.3.17".parse().unwrap();
        install(&version).await.unwrap();
        let zksolc_path = version_binary(version.to_string().as_str());
        let output = Command::new(zksolc_path).arg("--version").output().unwrap();
        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("1.3.17"));
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    fn blocking_test_latest() {
        blocking_install(&LATEST).unwrap();
        let zksolc_path = version_binary(LATEST.to_string().as_str());
        let output = Command::new(zkzksolc_path).arg("--version").output().unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains(&LATEST.to_string()));
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    fn blocking_test_version() {
        let version = "1.3.17".parse().unwrap();
        blocking_install(&version).unwrap();
        let zksolc_path = version_binary(version.to_string().as_str());
        let output = Command::new(zksolc_path).arg("--version").output().unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("1.3.17"));
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn can_install_parallel() {
        let version: Version = "1.3.17".parse().unwrap();
        let cloned_version = version.clone();
        let t = std::thread::spawn(move || blocking_install(&cloned_version));
        blocking_install(&version).unwrap();
        t.join().unwrap().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_install_parallel_async() {
        let version: Version = "1.3.17".parse().unwrap();
        let cloned_version = version.clone();
        let t = tokio::task::spawn(async move { install(&cloned_version).await });
        install(&version).await.unwrap();
        t.await.unwrap().unwrap();
    }

    // ensures we can download the latest universal zksolc for apple silicon
    #[tokio::test(flavor = "multi_thread")]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    async fn can_install_latest_native_apple_silicon() {
        let zksolc = install(&LATEST).await.unwrap();
        let output = Command::new(zksolc).arg("--version").output().unwrap();
        let version = String::from_utf8_lossy(&output.stdout);
        assert!(version.contains("1.4.1"), "{}", version);
    }

    // ensures we can download the latest native zksolc for linux aarch64
    #[tokio::test(flavor = "multi_thread")]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    async fn can_download_latest_linux_aarch64() {
        let artifacts = all_releases(Platform::LinuxAarch64).await.unwrap();

        let artifact = artifacts.releases.get(&LATEST).unwrap();
        let download_url = artifact_url(
            Platform::LinuxAarch64,
            &LATEST,
            artifact.to_string().as_str(),
        )
        .unwrap();

        let checksum = artifacts.get_checksum(&LATEST).unwrap();

        let resp = reqwest::get(download_url).await.unwrap();
        assert!(resp.status().is_success());
        let binbytes = resp.bytes().await.unwrap();
        ensure_checksum(&binbytes, &LATEST, checksum).unwrap();
    }

    #[tokio::test]
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    async fn can_install_windows_zip_release() {
        let version = "1.3.17".parse().unwrap();
        install(&version).await.unwrap();
        let zksolc_path = version_binary(version.to_string().as_str());
        let output = Command::new(&zksolc_path).arg("--version").output().unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("1.3.17"));
    }
}
