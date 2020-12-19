extern crate anyhow;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;
extern crate simplelog;
extern crate structopt;
extern crate toml;

mod args;
mod tests;

use anyhow::{Context, Result};

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
    let mut opt = args::get_args();
    opt.verbosity = 3; // for now
    args::init_logger(&opt);

    trace!("Options: {:#?}", opt);

    let mut tests = Vec::with_capacity(opt.tests.len());
    for test_dir in &opt.tests {
        tests.push(
            tests::RootTest::from_dir(test_dir)
                .with_context(|| format!("load test from {:?}", test_dir))?,
        );
    }
    debug!("Finished loading tests");
    trace!("Tests: {:#?}", tests);

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
