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
use crossterm::style::Colorize;

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

    let test_dirs = if let Some(recurse) = opt.recurse {
        std::fs::read_dir(&recurse)
            .with_context(|| format!("recurse into {:?}", recurse))?
            .flat_map(|entry| entry.map(|entry| entry.path()))
            .collect()
    } else {
        opt.tests
    };

    let mut tests = Vec::with_capacity(test_dirs.len());
    for test_dir in test_dirs {
        tests.push(
            tests::RootTest::from_dir(&test_dir)
                .with_context(|| format!("load test from {:?}", test_dir))?,
        );
    }
    debug!("Finished loading tests");
    trace!("Tests: {:#?}", tests);

    println!("Running {} roottests\n", tests.len());
    let mut counts = results::Counts::default();
    let mut fails = Vec::new();
    for test in tests {
        print!("{} ... ", test.name);
        std::io::stdout().flush().unwrap();

        let result = test
            .run(opt.cleanup)
            .with_context(|| format!("run test {}", test.name))?;

        println!("{}", result.status());
        counts.update(&result);
        if !result.ok() {
            fails.push((test.name, result));
        }
    }

    if !fails.is_empty() {
        println!("\nfailures:");
        for (test, result) in fails {
            println!("\n--- {} ---", test.blue());
            result.print_details();
        }
    }

    println!("\n{}", counts);
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
