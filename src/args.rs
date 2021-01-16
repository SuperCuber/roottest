use simplelog::{ConfigBuilder, LevelFilter, LevelPadding, TermLogger, TerminalMode};
use structopt::StructOpt;

use std::path::PathBuf;

#[derive(Debug, StructOpt)]
pub(crate) struct Opt {
    /// Specify between 0 and 3 times to control debug information verbosity
    #[structopt(short, long = "verbose", parse(from_occurrences))]
    pub verbosity: usize,

    /// Run every folder in a directory as a test (can be given multiple times)
    #[structopt(short, long)]
    pub directory: Vec<PathBuf>,

    /// Save actual output and do not delete the temporary root/ directory after running the test in it
    #[structopt(short="C", long="no-cleanup", parse(from_flag = std::ops::Not::not))]
    pub cleanup: bool,

    /// Specify once for short output, specify twice for no output when tests pass
    #[structopt(short, long, parse(from_occurrences))]
    pub quiet: usize,

    /// An optional list of test folders
    pub tests: Vec<PathBuf>,

    /// Include tests with ignore = true
    #[structopt(short, long)]
    pub include_ignored: bool,
}

pub(crate) fn get_args() -> anyhow::Result<Opt> {
    let mut opt = Opt::from_args();
    opt.verbosity = std::cmp::min(opt.verbosity, 3);
    init_logger(&opt);
    Ok(opt)
}

pub(crate) fn init_logger(opt: &Opt) {
    TermLogger::init(
        match opt.verbosity {
            0 if opt.quiet == 0 => LevelFilter::Warn,
            0 => LevelFilter::Error,
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
