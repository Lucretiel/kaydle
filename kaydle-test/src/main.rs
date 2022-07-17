use std::{
    collections::HashMap,
    io::{self, Read},
};

use anyhow::Context;
use kaydle::serde::de::from_str;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename = "item")]
struct Item(i32, i32, char);

#[derive(Deserialize, Debug)]
enum Enum {
    #[serde(rename = "int")]
    Int(i32),

    #[serde(rename = "string")]
    String(String),
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct AnnotatedString {
    #[serde(rename = "$kaydle::annotation")]
    annotation: Option<String>,
    value: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Document {
    name: String,
    age: i32,
    key_value: HashMap<String, i32>,
    items: Vec<Item>,
    enums: Vec<Enum>,
    annotated_values: Vec<AnnotatedString>,
}

fn main() -> anyhow::Result<()> {
    let mut buf = String::new();

    io::stdin()
        .read_to_string(&mut buf)
        .context("Failed to read from stdin")?;

    let values: Document = from_str(&buf).context("Failed to deserialize")?;

    println!("{:#?}", values);

    Ok(())
}
