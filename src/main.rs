#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;
extern crate simplelog;
extern crate structopt;
extern crate toml;

mod args;
mod results;
mod tests;

use anyhow::{Context, Result};

use std::io::Write;

fn main() {
    match run() {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            display_error(e);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
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
    for test in tests {
        print!("{} ... ", test.name);
        std::io::stdout().flush().unwrap();
        let result = test
            .run()
            .with_context(|| format!("run test {}", test.name))?;
        counts.update(&result);
        println!("{}", result);
    }
    println!("{}", counts);

    Ok(())
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
