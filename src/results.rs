use std::collections::BTreeMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crossterm::style::Colorize;

use crate::difference::FileNodeDiff;

#[derive(Debug)]
pub enum RootTestResult {
    Ok,
    Ignored,
    Failed {
        stdout: TestFieldComparison<Vec<u8>, Vec<u8>>,
        stderr: TestFieldComparison<Vec<u8>, Vec<u8>>,
        status: TestFieldComparison<i32, i32>,
        root: TestFieldComparison<FileNode, FileNode>,
    },
}

#[derive(Debug)]
pub enum TestFieldComparison<L, R> {
    Identical,
    Differs(L, R),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileNode {
    File {
        contents: Vec<u8>,
        permissions: Permissions,
    },
    Directory {
        children: BTreeMap<PathBuf, FileNode>,
        permissions: Permissions,
    },
    SymbolicLink {
        target: PathBuf,
        permissions: Permissions,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permissions {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Default)]
pub struct Counts {
    ok: usize,
    failed: usize,
    ignored: usize,
}

impl RootTestResult {
    pub fn new(
        test: &crate::tests::RootTest,
        output: std::process::Output,
    ) -> Result<RootTestResult> {
        let status = output.status.code().expect("status code of process");
        let status = if status == test.params.expected_status {
            TestFieldComparison::Identical
        } else {
            TestFieldComparison::Differs(status, test.params.expected_status)
        };

        let stdout = if output.stdout == test.expected_stdout {
            TestFieldComparison::Identical
        } else {
            TestFieldComparison::Differs(output.stdout, test.expected_stdout.clone())
        };

        let stderr = if output.stderr == test.expected_stderr {
            TestFieldComparison::Identical
        } else {
            TestFieldComparison::Differs(output.stderr, test.expected_stderr.clone())
        };

        let root = FileNode::load_from(&test.root).context("load actual root")?;
        let root_after = FileNode::load_from(&test.root_after).context("load expected root")?;
        let root = if root == root_after {
            TestFieldComparison::Identical
        } else {
            TestFieldComparison::Differs(root, root_after)
        };

        Ok(RootTestResult::Failed {
            status,
            stdout,
            stderr,
            root,
        }
        .upgrade_to_ok())
    }

    fn upgrade_to_ok(self) -> RootTestResult {
        match self {
            RootTestResult::Ok => RootTestResult::Ok,
            RootTestResult::Failed {
                status,
                stdout,
                stderr,
                root,
            } if status.identical()
                && stdout.identical()
                && stderr.identical()
                && root.identical() =>
            {
                RootTestResult::Ok
            }
            failed => failed,
        }
    }

    pub fn ok(&self) -> bool {
        matches!(self, RootTestResult::Ok | RootTestResult::Ignored)
    }

    pub fn status(&self) -> crossterm::style::StyledContent<&'static str> {
        match self {
            RootTestResult::Ok => "ok".green(),
            RootTestResult::Failed { .. } => "FAILED".red(),
            RootTestResult::Ignored => "ignored".grey(),
        }
    }

    pub fn short_status(&self) -> crossterm::style::StyledContent<&'static str> {
        match self {
            RootTestResult::Ok => ".".white(),
            RootTestResult::Failed { .. } => "F".red(),
            RootTestResult::Ignored { .. } => "I".grey(),
        }
    }

    pub fn print_details(self) {
        match self {
            RootTestResult::Ok => panic!("printing details of ok result"),
            RootTestResult::Ignored => panic!("printing details of ignored result"),
            RootTestResult::Failed {
                stdout,
                stderr,
                status,
                root,
            } => {
                if let TestFieldComparison::Differs(actual, expected) = status {
                    println!(
                        "status differs: actual {}, expected {}",
                        actual.to_string().red(),
                        expected.to_string().green(),
                    );
                }

                output_diff(stdout, "stdout");
                output_diff(stderr, "stderr");

                if let TestFieldComparison::Differs(actual, expected) = root {
                    let diff = FileNodeDiff::from_file_nodes(actual, expected);
                    println!("root directory differs:");
                    trace!("FileNodeDiff: {:#?}", diff);
                    diff.print(0);
                }
            }
        }
    }
}

fn output_diff(output: TestFieldComparison<Vec<u8>, Vec<u8>>, name: &str) {
    if let TestFieldComparison::Differs(actual, expected) = output {
        println!(
            "{} differs: ({}, {})",
            name,
            "actual".red(),
            "expected".green()
        );
        match (String::from_utf8(actual), String::from_utf8(expected)) {
            (Ok(actual), Ok(expected)) => {
                let diff: Vec<_> = diff::lines(&actual, &expected)
                    .into_iter()
                    .map(crate::difference::to_owned_diff_result)
                    .collect();
                assert!(crate::difference::diff_nonempty(&diff));
                crate::difference::print_diff(diff, 3);
            }
            (_, _) => {
                println!(
                    "  Either actual {} or expected {} is invalid UTF-8",
                    name, name
                );
                println!(
                    "  Run again with --no-cleanup and check actual.{} and expected.{}",
                    name, name
                );
            }
        }
    }
}

impl std::fmt::Display for Counts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = if self.failed == 0 {
            "ok".green()
        } else {
            "FAILED".red()
        };
        write!(
            f,
            "roottest result: {}. {} ok, {} failed, and {} ignored.",
            result,
            self.ok.to_string().green(),
            if self.failed == 0 {
                "0".to_string().green()
            } else {
                self.failed.to_string().red()
            },
            self.ignored.to_string().grey(),
        )
    }
}

impl FileNode {
    fn load_from(path: impl AsRef<Path>) -> Result<FileNode> {
        let path = path.as_ref();
        if let Ok(target) = path.read_link() {
            Ok(FileNode::SymbolicLink {
                target,
                permissions: Permissions::load_from(path).context("load path's permissions")?,
            })
        } else if path.is_dir() {
            let children: Result<BTreeMap<PathBuf, FileNode>> = path
                .read_dir()
                .context("read dir")?
                .map(|e| {
                    e.context("get dir entry").and_then(|e| {
                        FileNode::load_from(&e.path())
                            .map(|r| (PathBuf::from(e.path().file_name().expect("file name")), r))
                    })
                })
                .collect();
            Ok(FileNode::Directory {
                children: children?,
                permissions: Permissions::load_from(path).context("load path's permissions")?,
            })
        // TODO: is there a bug hiding here if file doesn't exist?
        } else {
            Ok(FileNode::File {
                contents: std::fs::read(path)
                    .with_context(|| format!("read contents of {:?}", path))?,
                permissions: Permissions::load_from(path).context("load path's permissions")?,
            })
        }
    }

    pub(crate) fn node_type(&self) -> &'static str {
        match self {
            FileNode::File { .. } => "file",
            FileNode::Directory { .. } => "directory",
            FileNode::SymbolicLink { .. } => "symbolic link",
        }
    }
}

impl Permissions {
    fn load_from(path: impl AsRef<Path>) -> Result<Permissions> {
        let path = path.as_ref();
        let metadata = path.symlink_metadata().context("get metadata")?;
        Ok(Permissions {
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
        })
    }
}

impl<L, R> TestFieldComparison<L, R> {
    fn identical(&self) -> bool {
        matches!(self, TestFieldComparison::Identical)
    }
}

impl Counts {
    pub fn update(&mut self, result: &RootTestResult) {
        match result {
            RootTestResult::Ok => self.ok += 1,
            RootTestResult::Ignored => self.ignored += 1,
            RootTestResult::Failed { .. } => self.failed += 1,
        }
    }

    pub fn tests_passed(&self) -> bool {
        self.failed == 0
    }
}
