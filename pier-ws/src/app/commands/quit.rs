pub struct Quit {
    pub no_cwd_file: bool,
}

impl Quit {
    pub fn new(no_cwd_file: bool) -> Self {
        Self { no_cwd_file }
    }
}
