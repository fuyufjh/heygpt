use std::borrow::Cow;

use console::style;
use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::Helper;

/// The rustyline helper for interactive mode.
/// Currently it's mostly for highlighting the prompt correctly.
#[derive(Clone, Debug, Default)]
pub struct ReplHelper;

impl Helper for ReplHelper {}

impl Validator for ReplHelper {
    fn validate(&self, _ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
}

impl Completer for ReplHelper {
    type Candidate = String;
}

impl Hinter for ReplHelper {
    type Hint = String;
}

impl Highlighter for ReplHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        if let Some(idx) = prompt.find(" => ") {
            let (role, rest) = prompt.split_at(idx);
            Cow::Owned(format!("{}{}", style(role).bold().cyan(), rest))
        } else {
            Cow::Borrowed(prompt)
        }
    }
}
