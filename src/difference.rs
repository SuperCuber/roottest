use crossterm::style::Colorize;

use std::cmp::{max, min};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::results::FileNode;

pub type Diff = Vec<diff::Result<String>>;
pub type HunkDiff = Vec<(usize, usize, Diff)>;

#[derive(Debug)]
pub enum FileNodeDiff {
    Identical,
    Unexpected(&'static str),
    Missing(&'static str),
    DifferentType(&'static str, &'static str),
    FileDiffers {
        contents: Diff,
    },
    DirectoryDiffers {
        children: BTreeMap<PathBuf, FileNodeDiff>,
    },
    SymbolicLinkDiffers {
        target: (PathBuf, PathBuf),
    },
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
    pub fn from_file_nodes(actual: &FileNode, expected: &FileNode) -> FileNodeDiff {
        match (actual, expected) {
            (
                FileNode::File {
                    contents: actual_contents,
                },
                FileNode::File {
                    contents: expected_contents,
                },
            ) => {
                let diff: Diff = diff::lines(&actual_contents, &expected_contents)
                    .into_iter()
                    .map(to_owned_diff_result)
                    .collect();

                if diff_nonempty(&diff) {
                    FileNodeDiff::FileDiffers { contents: diff }
                } else {
                    FileNodeDiff::Identical
                }
            }
            (
                FileNode::SymbolicLink {
                    target: actual_target,
                },
                FileNode::SymbolicLink {
                    target: expected_target,
                },
            ) => {
                if actual_target == expected_target {
                    FileNodeDiff::Identical
                } else {
                    FileNodeDiff::SymbolicLinkDiffers {
                        target: (actual_target.into(), expected_target.into()),
                    }
                }
            }
            (
                FileNode::Directory {
                    children: actual_children,
                },
                FileNode::Directory {
                    children: expected_children,
                },
            ) => {
                let mut actual_children = actual_children.clone();
                let mut expected_children = expected_children.clone();

                let mut different_children = BTreeMap::new();

                let compared_children = actual_children
                    .keys()
                    .collect::<BTreeSet<_>>()
                    .intersection(&expected_children.keys().collect())
                    .cloned()
                    .cloned()
                    .collect::<BTreeSet<PathBuf>>();

                for compared_child in compared_children {
                    let actual_child = actual_children.remove(compared_child.as_path()).unwrap();
                    let expected_child =
                        expected_children.remove(compared_child.as_path()).unwrap();

                    match FileNodeDiff::from_file_nodes(&actual_child, &expected_child) {
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
                    FileNodeDiff::Identical
                } else {
                    FileNodeDiff::DirectoryDiffers {
                        children: different_children,
                    }
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
                println!("unexpected: actual {}, expected {}", node_type.red(), "nothing".green());
            }
            FileNodeDiff::Missing(node_type) => {
                println!("missing: actual {}, expected {}", "nothing".red(), node_type.green());
            }
            FileNodeDiff::DifferentType(actual, expected) => {
                println!("type differs: {} != {}", actual.red(), expected.green())
            }
            FileNodeDiff::FileDiffers { contents } => {
                println!(
                    "contents differ ({}, {})",
                    "actual".red(),
                    "expected".green()
                );
                print_diff(contents, 3);
            }
            FileNodeDiff::DirectoryDiffers { children } => {
                println!();
                for (child, diff) in children {
                    print!("{}{}: ", spaces, child.to_string_lossy().blue());
                    diff.print(indentation + 2);
                }
            }
            FileNodeDiff::SymbolicLinkDiffers { target } => {
                println!(
                    "symbolic link's target differs: actual {}, expected {}",
                    target.0.to_string_lossy().red(),
                    target.1.to_string_lossy().green()
                );
            }
        }
    }
}
