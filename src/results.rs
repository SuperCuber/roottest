use crossterm::style::Colorize;

#[derive(Debug)]
pub enum RootTestResult {
    Ok,
    Failed {
        stdout: TestFieldComparison<Vec<u8>, Vec<u8>>,
        stderr: TestFieldComparison<Vec<u8>, Vec<u8>>,
        status: TestFieldComparison<i32, i32>,
        root: TestFieldComparison<RootDirectory, RootDirectory>,
    },
}

#[derive(Debug)]
pub enum TestFieldComparison<L, R> {
    Identical,
    Differs(L, R),
}

#[derive(Debug)]
pub struct RootDirectory {}

#[derive(Debug, Default)]
pub struct Counts {
    ok: usize,
    failed: usize,
}

impl Counts {
    pub fn update(&mut self, result: &RootTestResult) {
        match result {
            RootTestResult::Ok => self.ok += 1,
            RootTestResult::Failed { .. } => self.failed += 1,
        }
    }
}

impl std::fmt::Display for RootTestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RootTestResult::Ok => write!(f, "{}", "ok".green()),
            RootTestResult::Failed { .. } => todo!(),
        }
    }
}

impl std::fmt::Display for Counts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = if self.failed == 0 {
            "ok".green()
        } else {
            "failed".red()
        };
        write!(
            f,
            "roottest result: {}. {} ok, {} failed.",
            result, self.ok, self.failed
        )
    }
}
