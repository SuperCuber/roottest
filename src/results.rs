#[derive(Debug)]
pub enum TestFieldComparison<L, R> {
    Identical,
    Differs(L, R)
}

#[derive(Debug)]
pub struct RootDirectory {
}

