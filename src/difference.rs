use crossterm::style::Colorize;

use std::cmp::{max, min};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::results::{FileNode, Permissions, TestFieldComparison};

pub type Diff = Vec<diff::Result<String>>;
pub type HunkDiff = Vec<(usize, usize, Diff)>;

#[derive(Debug)]
pub enum FileNodeDiff {
    Identical,
    Unexpected(&'static str),
    Missing(&'static str),
    DifferentType(&'static str, &'static str),
    FileDiffers {
        contents: Option<FileDiff>,
        permissions: Option<PermissionsDiff>,
    },
    DirectoryDiffers {
        children: Option<BTreeMap<PathBuf, FileNodeDiff>>,
        permissions: Option<PermissionsDiff>,
    },
    SymbolicLinkDiffers {
        target: Option<(PathBuf, PathBuf)>,
        permissions: Option<PermissionsDiff>,
    },
}

#[derive(Debug)]
pub enum FileDiff {
    Diff(Diff),
    Binary,
}

#[derive(Debug)]
pub struct PermissionsDiff {
    mode: TestFieldComparison<u32, u32>,
    uid: TestFieldComparison<u32, u32>,
    gid: TestFieldComparison<u32, u32>,
}

pub fn to_owned_diff_result(from: diff::Result<&str>) -> diff::Result<String> {
    match from {
        diff::Result::Left(s) => diff::Result::Left(s.to_string()),
        diff::Result::Right(s) => diff::Result::Right(s.to_string()),
        diff::Result::Both(s1, s2) => diff::Result::Both(s1.to_string(), s2.to_string()),
    }
}

pub fn diff_nonempty(diff: &[diff::Result<String>]) -> bool {
    for line in diff {
        match line {
            diff::Result::Both(..) => {}
            _ => {
                return true;
            }
        }
    }
    false
}

fn hunkify_diff(diff: Diff, extra_lines: usize) -> HunkDiff {
    let mut hunks = vec![];

    let mut left_line_number: usize = 1;
    let mut right_line_number: usize = 1;

    let mut current_hunk = None;

    for (position, line) in diff.iter().enumerate() {
        match line {
            diff::Result::Left(_) | diff::Result::Right(_) => {
                // The central part of a hunk
                if current_hunk.is_none() {
                    current_hunk = Some((left_line_number, right_line_number, vec![]));
                }
                current_hunk.as_mut().unwrap().2.push(line.clone());
            }
            diff::Result::Both(_, _) => {
                if diff[position..=min(position + extra_lines, diff.len() - 1)]
                    .iter()
                    .any(is_different)
                {
                    // There's a hunk soon - but we might already be in a hunk
                    if current_hunk.is_none() {
                        current_hunk = Some((left_line_number, right_line_number, vec![]));
                    }
                    current_hunk.as_mut().unwrap().2.push(line.clone());
                } else if diff[position.saturating_sub(extra_lines)..position]
                    .iter()
                    .any(is_different)
                {
                    // We're just after a hunk
                    current_hunk.as_mut().unwrap().2.push(line.clone());
                } else if let Some(hunk) = current_hunk.take() {
                    // We're finished with the current hunk
                    hunks.push(hunk);
                }
            }
        }

        // Keep track of line numbers
        match line {
            diff::Result::Left(_) => {
                left_line_number += 1;
            }
            diff::Result::Right(_) => {
                right_line_number += 1;
            }
            diff::Result::Both(_, _) => {
                left_line_number += 1;
                right_line_number += 1;
            }
        }
    }

    // Last hunk - in case the last line is included in a hunk, it was never added
    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    hunks
}

fn is_different(diff: &diff::Result<String>) -> bool {
    !matches!(diff, diff::Result::Both(..))
}

fn print_hunk(mut left_line: usize, mut right_line: usize, hunk: Diff, max_digits: usize) {
    for line in hunk {
        match line {
            diff::Result::Left(l) => {
                println!(
                    " {:>width$} | {:>width$} | {}",
                    left_line.to_string().red(),
                    "",
                    l.red(),
                    width = max_digits
                );
                left_line += 1;
            }
            diff::Result::Both(l, _) => {
                println!(
                    " {:>width$} | {:>width$} | {}",
                    left_line.to_string().dark_grey(),
                    right_line.to_string().dark_grey(),
                    l,
                    width = max_digits
                );
                left_line += 1;
                right_line += 1;
            }
            diff::Result::Right(r) => {
                println!(
                    " {:>width$} | {:>width$} | {}",
                    "",
                    right_line.to_string().green(),
                    r.green(),
                    width = max_digits
                );
                right_line += 1;
            }
        }
    }
}

pub fn print_diff(diff: Diff, extra_lines: usize) {
    let mut diff = hunkify_diff(diff, extra_lines);

    let last_hunk = diff.pop().expect("at least one hunk");
    let max_possible_line = max(last_hunk.0, last_hunk.1) + last_hunk.2.len();
    let max_possible_digits = max_possible_line.to_string().len(); // yes I could log10, whatever

    for hunk in diff {
        print_hunk(hunk.0, hunk.1, hunk.2, max_possible_digits);
        println!();
    }

    print_hunk(last_hunk.0, last_hunk.1, last_hunk.2, max_possible_digits);
}

impl FileNodeDiff {
    pub fn from_file_nodes(actual: FileNode, expected: FileNode) -> FileNodeDiff {
        match (actual, expected) {
            (
                FileNode::File {
                    contents: actual_contents,
                    permissions: actual_permissions,
                },
                FileNode::File {
                    contents: expected_contents,
                    permissions: expected_permissions,
                },
            ) => {
                let contents = if actual_contents == expected_contents {
                    None
                } else {
                    match (
                        String::from_utf8(actual_contents),
                        String::from_utf8(expected_contents),
                    ) {
                        (Ok(actual_contents), Ok(expected_contents)) => {
                            let diff: Diff = diff::lines(&actual_contents, &expected_contents)
                                .into_iter()
                                .map(to_owned_diff_result)
                                .collect();

                            assert!(diff_nonempty(&diff));
                            Some(FileDiff::Diff(diff))
                        }
                        (_, _) => Some(FileDiff::Binary),
                    }
                };

                let permissions = if actual_permissions == expected_permissions {
                    None
                } else {
                    Some(PermissionsDiff::from_permissions(
                        actual_permissions,
                        expected_permissions,
                    ))
                };

                match (contents, permissions) {
                    (None, None) => FileNodeDiff::Identical,
                    (contents, permissions) => FileNodeDiff::FileDiffers {
                        contents,
                        permissions,
                    },
                }
            }
            (
                FileNode::SymbolicLink {
                    target: actual_target,
                    permissions: actual_permissions,
                },
                FileNode::SymbolicLink {
                    target: expected_target,
                    permissions: expected_permissions,
                },
            ) => {
                let target = if actual_target == expected_target {
                    None
                } else {
                    Some((actual_target, expected_target))
                };

                let permissions = if actual_permissions == expected_permissions {
                    None
                } else {
                    Some(PermissionsDiff::from_permissions(
                        actual_permissions,
                        expected_permissions,
                    ))
                };

                match (target, permissions) {
                    (None, None) => FileNodeDiff::Identical,
                    (target, permissions) => FileNodeDiff::SymbolicLinkDiffers {
                        target,
                        permissions,
                    },
                }
            }
            (
                FileNode::Directory {
                    children: mut actual_children,
                    permissions: actual_permissions,
                },
                FileNode::Directory {
                    children: mut expected_children,
                    permissions: expected_permissions,
                },
            ) => {
                let children = {
                    let mut different_children = BTreeMap::new();

                    let compared_children = actual_children
                        .keys()
                        .collect::<BTreeSet<_>>()
                        .intersection(&expected_children.keys().collect())
                        .cloned()
                        .cloned()
                        .collect::<BTreeSet<PathBuf>>();

                    for compared_child in compared_children {
                        let actual_child =
                            actual_children.remove(compared_child.as_path()).unwrap();
                        let expected_child =
                            expected_children.remove(compared_child.as_path()).unwrap();

                        match FileNodeDiff::from_file_nodes(actual_child, expected_child) {
                            FileNodeDiff::Identical => {}
                            diff => {
                                different_children.insert(compared_child, diff);
                            }
                        }
                    }

                    for (missing_child, missing_child_value) in expected_children {
                        different_children.insert(
                            missing_child,
                            FileNodeDiff::Missing(missing_child_value.node_type()),
                        );
                    }

                    for (unexpected_child, unexpected_child_value) in actual_children {
                        different_children.insert(
                            unexpected_child,
                            FileNodeDiff::Unexpected(unexpected_child_value.node_type()),
                        );
                    }

                    if different_children.is_empty() {
                        None
                    } else {
                        Some(different_children)
                    }
                };

                let permissions = if actual_permissions == expected_permissions {
                    None
                } else {
                    Some(PermissionsDiff::from_permissions(
                        actual_permissions,
                        expected_permissions,
                    ))
                };

                match (children, permissions) {
                    (None, None) => FileNodeDiff::Identical,
                    (children, permissions) => FileNodeDiff::DirectoryDiffers {
                        children,
                        permissions,
                    },
                }
            }
            (actual, expected) => {
                FileNodeDiff::DifferentType(actual.node_type(), expected.node_type())
            }
        }
    }

    pub fn print(self, indentation: usize) {
        let spaces: String = (0..indentation).map(|_| ' ').collect();
        match self {
            FileNodeDiff::Identical => unreachable!("printing identical node"),
            FileNodeDiff::Unexpected(node_type) => {
                println!(
                    "{}unexpected: actual {}, expected {}",
                    spaces,
                    node_type.red(),
                    "nothing".green()
                );
            }
            FileNodeDiff::Missing(node_type) => {
                println!(
                    "{}missing: actual {}, expected {}",
                    spaces,
                    "nothing".red(),
                    node_type.green()
                );
            }
            FileNodeDiff::DifferentType(actual, expected) => {
                println!(
                    "{}type differs: {} != {}",
                    spaces,
                    actual.red(),
                    expected.green()
                )
            }
            FileNodeDiff::FileDiffers {
                contents,
                permissions,
            } => {
                if let Some(permissions) = permissions {
                    println!("{}permissions differ:", spaces);
                    permissions.print(indentation + 2);
                }

                if let Some(contents) = contents {
                    println!(
                        "{}contents differ ({}, {})",
                        spaces,
                        "actual".red(),
                        "expected".green()
                    );
                    match contents {
                        FileDiff::Diff(diff) => print_diff(diff, 3),
                        FileDiff::Binary => {
                            println!("  Either actual file or expected file is invalid UTF-8");
                            println!("  Run again with --no-cleanup and check the contents of root/ and root_after/");
                        }
                    }
                }
            }
            FileNodeDiff::DirectoryDiffers {
                children,
                permissions,
            } => {
                if let Some(permissions) = permissions {
                    println!("{}permissions differ:", spaces);
                    permissions.print(indentation + 2);
                }
                if let Some(children) = children {
                    for (child, diff) in children {
                        println!("{}{}:", spaces, child.to_string_lossy().blue());
                        diff.print(indentation + 2);
                    }
                }
            }
            FileNodeDiff::SymbolicLinkDiffers {
                target,
                permissions,
            } => {
                if let Some(permissions) = permissions {
                    println!("{}permissions differ:", spaces);
                    permissions.print(indentation + 2);
                }

                if let Some(target) = target {
                    println!(
                        "{}symbolic link's target differs: actual {}, expected {}",
                        spaces,
                        target.0.to_string_lossy().red(),
                        target.1.to_string_lossy().green()
                    );
                }
            }
        }
    }
}

impl PermissionsDiff {
    fn from_permissions(actual: Permissions, expected: Permissions) -> Self {
        let mode = if actual.mode == expected.mode {
            TestFieldComparison::Identical
        } else {
            TestFieldComparison::Differs(actual.mode, expected.mode)
        };
        let uid = if actual.uid == expected.uid {
            TestFieldComparison::Identical
        } else {
            TestFieldComparison::Differs(actual.uid, expected.uid)
        };
        let gid = if actual.gid == expected.gid {
            TestFieldComparison::Identical
        } else {
            TestFieldComparison::Differs(actual.gid, expected.gid)
        };

        PermissionsDiff { mode, uid, gid }
    }

    fn print(&self, indentation: usize) {
        let spaces: String = (0..indentation).map(|_| ' ').collect();
        if let TestFieldComparison::Differs(actual, expected) = self.mode {
            println!(
                "{}mode: actual {}, expected {}",
                spaces,
                format!("{:o}", actual & 0o777).red(),
                format!("{:o}", expected & 0o777).green()
            );
        }
        if let TestFieldComparison::Differs(actual, expected) = self.uid {
            println!(
                "{}uid: actual {}, expected {}",
                spaces,
                actual.to_string().red(),
                expected.to_string().green()
            );
        }
        if let TestFieldComparison::Differs(actual, expected) = self.gid {
            println!(
                "{}gid: actual {}, expected {}",
                spaces,
                actual.to_string().red(),
                expected.to_string().green()
            );
        }
    }
}
