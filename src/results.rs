use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crossterm::style::Colorize;

#[derive(Debug)]
pub enum RootTestResult {
    Ok,
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

#[derive(Debug, PartialEq, Eq)]
pub enum FileNode {
    File {
        contents: String,
        // something like
        // metadata: (),
        // permissions: (),
        // uid: (),
        // gid: (),
    },
    Directory {
        children: BTreeMap<PathBuf, FileNode>,
        // metadata: (),
    },
    SymbolicLink {
        target: PathBuf,
    },
}

#[derive(Debug, Default)]
pub struct Counts {
    ok: usize,
    failed: usize,
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
}

impl FileNode {
    fn load_from(path: impl AsRef<Path>) -> Result<FileNode> {
        let path = path.as_ref();
        if let Ok(target) = path.read_link() {
            Ok(FileNode::SymbolicLink { target })
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
            })
        } else {
            Ok(FileNode::File {
                contents: std::fs::read_to_string(path)
                    .with_context(|| format!("read contents of {:?}", path))?,
            })
        }
    }
}

impl RootTestResult {
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
            RootTestResult::Failed { .. } => self.failed += 1,
        }
    }
}

impl RootTestResult {
    pub fn print(&self) {
        match self {
            RootTestResult::Ok => println!("{}", "ok".green()),
            RootTestResult::Failed {
                stdout,
                stderr,
                status,
                root,
            } => {
                println!("{}", "FAILED".red());
                if let TestFieldComparison::Differs(actual, expected) = status {
                    println!(
                        "status differs: actual {}, expected {}",
                        actual.to_string().red(),
                        expected.to_string().green(),
                    );
                }
                if let TestFieldComparison::Differs(actual, expected) = stdout {
                    println!(
                        "stdout differs: ({}, {})",
                        "actual".red(),
                        "expected".green()
                    );
                    let actual = String::from_utf8_lossy(actual);
                    let expected = String::from_utf8_lossy(expected);
                    let diff: Vec<_> = diff::lines(&actual, &expected)
                        .into_iter()
                        .map(crate::difference::to_owned_diff_result)
                        .collect();
                    assert!(crate::difference::diff_nonempty(&diff));
                    crate::difference::print_diff(diff, 3);
                }
                if let TestFieldComparison::Differs(actual, expected) = stderr {
                    println!(
                        "stderr differs: ({}, {})",
                        "actual".red(),
                        "expected".green()
                    );
                    let actual = String::from_utf8_lossy(actual);
                    let expected = String::from_utf8_lossy(expected);
                    let diff: Vec<_> = diff::lines(&actual, &expected)
                        .into_iter()
                        .map(crate::difference::to_owned_diff_result)
                        .collect();
                    assert!(crate::difference::diff_nonempty(&diff));
                    crate::difference::print_diff(diff, 3);
                }
                if let TestFieldComparison::Differs(_actual, _expected) = root {
                    todo!()
                }
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
            "roottest result: {}. {} ok, {} failed.",
            result, self.ok, self.failed
        )
    }
}
