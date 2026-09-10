use std::io::Cursor;

use skim::{Skim, options::SkimOptionsBuilder, prelude::SkimItemReader};
use sqlx::Row;

use crate::database::get_db_pool;

pub fn selection(title: &str, candidate: String) -> Result <String, Box<dyn std::error::Error>> { 
    let mut result = String::new();
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(candidate));

    let options = SkimOptionsBuilder::default()
        .multi(false)
        .prompt("Search > ".to_string())
        .header(title)
        .build()
        .unwrap();

    let skim_output = Skim::run_with(options, Some(items));
    if let Ok(output) = skim_output {
        if !output.is_abort {
            result = output.selected_items[0].item.text().to_string();
        }
    }

    Ok(result)
}

pub fn multi_selection(title: &str, candidate: String) -> Result <Vec<String>, Box<dyn std::error::Error>> { 
    let mut results = Vec::new();
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(candidate));

    let options = SkimOptionsBuilder::default()
        .multi(true)
        .prompt("Search > ".to_string())
        .header(title)
        .build()
        .unwrap();

    let skim_output = Skim::run_with(options, Some(items));
    if let Ok(output) = skim_output {
        if !output.is_abort {
            for selected_item in output.selected_items {
                results.push(selected_item.item.text().to_string());
            }
        }
    }

    Ok(results)
}