use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use crate::results::{RootDirectory, TestFieldComparison};

use anyhow::{Context, Result};

#[derive(Debug, Deserialize)]
pub struct RootTestParams {
    cd: PathBuf,
    run: Option<String>,
    expected_status: i32,
}

#[derive(Debug)]
pub struct RootTest {
    params: RootTestParams,
    stdin: Vec<u8>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    environment: BTreeMap<String, String>,
    // Directories
    root_before: PathBuf,
    root_after: PathBuf,
}

#[derive(Debug)]
pub struct RootTestResult {
    pub stdout: TestFieldComparison<Vec<u8>, Vec<u8>>,
    pub stderr: TestFieldComparison<Vec<u8>, Vec<u8>>,
    pub status: TestFieldComparison<i32, i32>,
    pub root: TestFieldComparison<RootDirectory, RootDirectory>
}

impl RootTest {
    pub fn from_dir(dir: &Path) -> Result<RootTest> {
        debug!("Loading test from {:?}", dir);

        let params: RootTestParams = toml::from_str(
            &read_to_string(dir.join("Roottest.toml")).context("read roottest.toml")?,
        )
        .context("parse roottest.toml")?;
        trace!("Params: {:#?}", params);

        if params.run.is_none() {
            todo!("read run.sh");
        }

        let stdin = std::fs::read(dir.join("input.stdin")).context("load stdin")?;
        trace!("Stdin: {:#?}", stdin);
        let stdout = std::fs::read(dir.join("expected.stdout")).context("load stdout")?;
        trace!("Stdout: {:#?}", stdout);
        let stderr = std::fs::read(dir.join("expected.stderr")).context("load stderr")?;
        trace!("Stderr: {:#?}", stderr);

        let environment = toml::from_str(
            &read_to_string(dir.join("environment.toml")).context("read environment.toml")?,
        )
        .context("parse environment.toml")?;
        trace!("Environment: {:#?}", environment);

        Ok(RootTest {
            params,
            stdin,
            stdout,
            stderr,
            environment,
            root_before: dir.join("root_before"),
            root_after: dir.join("root_after"),
        })
    }

    pub fn run() -> Result<RootTestResult> {
        todo!()
    }
}
