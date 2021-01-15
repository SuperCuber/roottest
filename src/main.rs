#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod args;
mod difference;
mod results;
mod tests;

use anyhow::{Context, Result};
use crossterm::style::Styler;

use std::io::Write;

fn main() {
    match run() {
        Ok(true) => std::process::exit(0),
        Ok(false) => std::process::exit(1),
        Err(e) => {
            display_error(e);
            std::process::exit(2);
        }
    }
}

fn run() -> Result<bool> {
    let opt = args::get_args().context("parse arguments")?;
    trace!("Options: {:#?}", opt);

    let mut test_dirs = opt.tests.clone();
    for dir in opt.directory {
        test_dirs.extend(
            std::fs::read_dir(&dir)
                .with_context(|| format!("recurse into {:?}", dir))?
                .flat_map(|entry| entry.map(|entry| entry.path())),
        )
    }

    info!("Loading {} tests", test_dirs.len());
    trace!("Test directories: {:#?}", test_dirs);

    let mut tests = Vec::with_capacity(test_dirs.len());
    for test_dir in test_dirs {
        tests.push(
            tests::RootTest::from_dir(&test_dir)
                .with_context(|| format!("load test from {:?}", test_dir))?,
        );
    }
    trace!("Tests: {:#?}", tests);

    if opt.quiet == 0 {
        println!("Running {} roottests\n", tests.len());
    }

    let mut counts = results::Counts::default();
    let mut fails = Vec::new();
    for test in tests {
        if opt.quiet == 0 {
            print!("{} ... ", test.name);
            std::io::stdout().flush().unwrap();
        }

        let result = test
            .run(opt.cleanup, opt.include_ignored)
            .with_context(|| format!("run test {}", test.name))?;

        if opt.quiet == 0 {
            println!("{}", result.status());
        } else if opt.quiet == 1 {
            print!("{}", result.short_status());
            std::io::stdout().flush().unwrap();
        }

        counts.update(&result);
        if !result.ok() {
            fails.push((test.name, result));
        }
    }

    if opt.quiet == 1 {
        // Break line after dots
        println!();
    }

    if !fails.is_empty() {
        if opt.quiet <= 1 {
            println!();
        }
        println!("failures:");

        for (test, result) in fails {
            println!("\n--- {} ---", test.bold());
            result.print_details();
        }
    }

    if opt.quiet <= 1 || !counts.tests_passed() {
        println!("\n{}", counts);
    }

    Ok(counts.tests_passed())
}

pub(crate) fn display_error(error: anyhow::Error) {
    let mut chain = error.chain();
    let mut error_message = format!("Failed to {}\nCaused by:\n", chain.next().unwrap());

    for e in chain {
        error_message.push_str(&format!("    {}\n", e));
    }
    // Remove last \n
    error_message.pop();

    error!("{}", error_message);
}
