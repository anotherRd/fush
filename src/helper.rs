use std::io::{self, Write};

pub fn read_from_input(caption: &str, default: Option<&str>, choices: Vec<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    let mut trimmed_input = "";

    while trimmed_input == "" {
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
            } else {
                println!("{caption} is required");
            }
        } else if choices.len() > 0 {
            // if choices available
            if !choices.contains(&trimmed_input) {
                let choice_values = choices.join("/");
                println!("{caption} value must be [{choice_values}]");
                trimmed_input = "";
            }
        }
    }

    Ok(trimmed_input.to_string())
}


pub fn split_selected(selected: &str) -> (String, String) {
    let prefix: Vec<&str> = selected.split(": ").collect();
    let selected = selected.replacen(&format!("{}: ", &prefix[0]), "", 1);

    (prefix[0].to_string(), selected)
}