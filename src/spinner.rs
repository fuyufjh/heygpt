use console::Term;

/// An auxiliary struct to handle the spinner
///
/// A spinner will be shown when this struct is created, and will be removed when it is dropped.
pub struct Spinner(spinners::Spinner);

impl Spinner {
    pub fn new() -> Self {
        use spinners::{Spinner, Spinners};
        let sp = Spinner::new(Spinners::SimpleDotsScrolling, "".into());
        Self(sp)
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.0.stop();
        Term::stdout().clear_line().unwrap();
    }
}
