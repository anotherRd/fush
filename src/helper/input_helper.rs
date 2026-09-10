use std::{fs, io::{self, Write}};
use crate::{config::{blacklisted_key_name, key_dir}, helper::general_helper::custom_print};
use rustyline::{
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::Hinter,
    validate::Validator,
    Editor, Helper,
};

pub fn read_from_input(
    caption: &str,
    default: Option<&str>,
    choices: Vec<&str>,
    required: bool
) -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    let mut trimmed_input = "";
    let mut continue_loop = true;

    while continue_loop {
        continue_loop = false;
        // read from input
        print!("{caption}: ");
        io::stdout().flush()?;
        input.clear();
        io::stdin().read_line(&mut input)?;
        trimmed_input = input.trim();

        // if empty
        if trimmed_input == "" {
            // set to default if available
            if let Some(default_value) = default {
                trimmed_input = default_value;
            } else if required{
                custom_print("warning", &format!("{caption} is required"));
                continue_loop = true;
            }
        } else if choices.len() > 0 {
            // if choices available
            if !choices.contains(&trimmed_input) {
                let choice_values = choices.join("/");
                custom_print("warning", &format!("{caption} value must be [{choice_values}]"));
                continue_loop = true;
            }
        }
    }

    Ok(trimmed_input.to_string())
}

pub struct AutocompleteHelper {
    pub candidates: Vec<String>,
}

impl Completer for AutocompleteHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let input = &line[..pos];

        let matches = self
            .candidates
            .iter()
            .filter(|candidate| candidate.starts_with(input))
            .map(|candidate| Pair {
                display: candidate.clone(),
                replacement: candidate.clone(),
            })
            .collect();

        Ok((0, matches))
    }
}

impl Hinter for AutocompleteHelper {
    type Hint = String;
}

impl Highlighter for AutocompleteHelper {}
impl Validator for AutocompleteHelper {}
impl Helper for AutocompleteHelper {}

pub fn auto_complete_key(
    caption: &str,
    default: Option<&str>,
    choices: Vec<&str>,
    required: bool
) -> Result<String, Box<dyn std::error::Error>> {
    let blacklisted_key_name = blacklisted_key_name();
    let candidates: Vec<String> = fs::read_dir(&key_dir()?)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .filter(|entry| {
            !entry.file_name().to_string_lossy().ends_with(".pub")
        })
        .filter(|entry| {
            !blacklisted_key_name.contains(&entry.file_name().to_string_lossy().as_ref())
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    let helper = AutocompleteHelper { candidates };

    let mut rl = Editor::new()?;
    rl.set_helper(Some(helper));

    let mut input;
    let mut trimmed_input = "";
    let mut continue_loop = true;

    while continue_loop {
        continue_loop = false;
        // read from input
        input = rl.readline(&format!("{caption}: "))?;
        trimmed_input = &input;

        // if empty
        if trimmed_input == "" {
            // set to default if available
            if let Some(default_value) = default {
                trimmed_input = default_value;
            } else if required{
                custom_print("warning", &format!("{caption} is required"));
                continue_loop = true;
            }
        } else if trimmed_input.ends_with(".pub") {
            custom_print("info", &format!("key name ended with .pub is not allowed"));
            continue_loop = true;
        } else if choices.len() > 0 {
            // if choices available
            if !choices.contains(&trimmed_input) {
                let choice_values = choices.join("/");
                custom_print("warning", &format!("{caption} value must be [{choice_values}]"));
                continue_loop = true;
            }
        } else if blacklisted_key_name.contains(&trimmed_input) {
            continue_loop = true;
            custom_print("warning", &format!("Key name can't be on of [{}]", &blacklisted_key_name.join(", ")));
        }
    }

    Ok(trimmed_input.to_string())
}