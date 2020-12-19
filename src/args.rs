use simplelog::{ConfigBuilder, LevelFilter, LevelPadding, TermLogger, TerminalMode};
use structopt::StructOpt;

use std::path::PathBuf;

#[derive(Debug, StructOpt)]
pub(crate) struct Opt {
    #[structopt(short, long = "verbose", parse(from_occurrences))]
    pub verbosity: usize,

    /// Run every folder in this directory as a test
    #[structopt(short, long)]
    pub recurse: Option<PathBuf>,

    /// A list of test folders
    pub tests: Vec<PathBuf>,
}

pub(crate) fn get_args() -> anyhow::Result<Opt> {
    let mut opt = Opt::from_args();
    opt.verbosity = std::cmp::min(opt.verbosity, 3);
    init_logger(&opt);
    if opt.recurse.is_some() && !opt.tests.is_empty() {
        bail!("cannot specify tests if --recurse is given");
    }
    Ok(opt)
}

pub(crate) fn init_logger(opt: &Opt) {
    TermLogger::init(
        match opt.verbosity {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            3 => LevelFilter::Trace,
            _ => unreachable!(),
        },
        ConfigBuilder::new()
            .set_time_level(LevelFilter::Off)
            .set_location_level(LevelFilter::Debug)
            .set_target_level(LevelFilter::Off)
            .set_thread_level(LevelFilter::Off)
            .set_level_padding(LevelPadding::Left)
            .build(),
        TerminalMode::Mixed,
    )
    .expect("initialize logger");
}
