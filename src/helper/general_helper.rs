use crate::config::requirements;


pub fn check_requirement() -> Result<(), Box<dyn std::error::Error>> {
    // get requirement
    let (mandatory, optional) = requirements();
    
    let optinal: Vec<_> = optional
        .into_iter()
        .filter(|command| which::which(command).is_err())
        .collect();

    for command in &optinal {
        custom_print("warning", &format!("missing: {command}"));
    }

    let mandatory: Vec<_> = mandatory
        .into_iter()
        .filter(|command| which::which(command).is_err())
        .collect();

    if mandatory.len() > 0 {
        return Err(format!("Error: missing requirements: [{}]", mandatory.join(", ")).into());
    }

    Ok(())
}

pub fn split_server_address(address: &str) -> (String, String, String) {
    // split address
    let (user, rest) = address.split_once('@').unwrap();
    let (host, port) = rest.split_once(':').unwrap();

    (user.to_string(), host.to_string(), port.to_string())
}

pub fn custom_print(message_type: &str, message: &str) {
    match message_type {
        "info" => {
            println!("Info: {message}");
        },
        "success" => {
            println!("Success: {message}");
        },
        "warning" => {
            println!("Warning: {message}");
        },
        "error" =>  {
            eprintln!("Error: {message}");

        },
        _ => {
            println!("{message}");
        }
    }
}

pub fn split_selected(selected: &str) -> (String, String) {
    let prefix: Vec<&str> = selected.split(": ").collect();
    let selected = selected.replacen(&format!("{}: ", &prefix[0]), "", 1);

    (prefix[0].to_string(), selected)
}