#[derive(Debug)]
pub struct RootTestResult {
    pub stdout: TestFieldComparison<Vec<u8>, Vec<u8>>,
    pub stderr: TestFieldComparison<Vec<u8>, Vec<u8>>,
    pub status: TestFieldComparison<i32, i32>,
    pub root: TestFieldComparison<RootDirectory, RootDirectory>,
}

#[derive(Debug)]
pub enum TestFieldComparison<L, R> {
    Identical,
    Differs(L, R),
}

#[derive(Debug)]
pub struct RootDirectory {}
