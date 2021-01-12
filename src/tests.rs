use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use crate::results::RootTestResult;

use anyhow::{Context, Result};

#[derive(Debug, Deserialize)]
pub struct RootTestParams {
    cd: PathBuf,
    run: String,
    expected_status: i32,
}

#[derive(Debug)]
pub struct RootTest {
    pub(crate) name: String,
    params: RootTestParams,
    stdin: Vec<u8>,
    expected_stdout: Vec<u8>,
    expected_stderr: Vec<u8>,
    environment: BTreeMap<String, String>,
    // Directories
    root_before: PathBuf,
    root: PathBuf,
    root_after: PathBuf,
}

impl RootTest {
    pub fn from_dir(dir: &Path) -> Result<RootTest> {
        debug!("Loading test from {:?}", dir);

        let params: RootTestParams = toml::from_str(
            &read_to_string(dir.join("Roottest.toml")).context("read roottest.toml")?,
        )
        .context("parse roottest.toml")?;
        trace!("Params: {:#?}", params);

        let stdin = std::fs::read(dir.join("input.stdin")).context("load stdin")?;
        trace!("Stdin: {:#?}", stdin);
        let expected_stdout = std::fs::read(dir.join("expected.stdout")).context("load stdout")?;
        trace!("Stdout: {:#?}", expected_stdout);
        let expected_stderr = std::fs::read(dir.join("expected.stderr")).context("load stderr")?;
        trace!("Stderr: {:#?}", expected_stderr);

        let environment = toml::from_str(
            &read_to_string(dir.join("environment.toml")).context("read environment.toml")?,
        )
        .context("parse environment.toml")?;
        trace!("Environment: {:#?}", environment);

        Ok(RootTest {
            name: dir
                .file_name()
                .context("get name of test's directory")?
                .to_string_lossy()
                .into(),
            params,
            stdin,
            expected_stdout,
            expected_stderr,
            environment,
            root_before: dir.join("root_before"),
            root: dir.join("root"),
            root_after: dir.join("root_after"),
        })
    }

    pub fn run(&self) -> Result<RootTestResult> {
        if log_enabled!(log::Level::Debug) {
            // newline after the 3 dots
            println!();
        }

        debug!(
            "Cleaning up previous root at {:?} if it exists...",
            self.root
        );
        let _ = std::fs::remove_dir_all(&self.root);

        debug!("Copying {:?} to {:?}...", self.root_before, self.root);
        let cp_success = std::process::Command::new("cp")
            .arg("-r")
            .arg(&self.root_before)
            .arg(&self.root)
            .output()
            .context("run cp self.root_before self.root")?
            .status
            .success();
        ensure!(
            cp_success,
            "failed to run cp -r {:?} {:?}",
            self.root_before,
            self.root
        );

        debug!("Launching chrooted process");
        let process_status = std::process::Command::new("fakechroot")
            .arg("chroot")
            .arg(&self.root)
            .arg("sh")
            .arg("-c")
            .arg(format!("cd {:?} && {}", self.params.cd, self.params.run))
            .output()
            .context("run test command in chroot")?;
        dbg!(process_status);

        Ok(RootTestResult::Ok)
    }
}
