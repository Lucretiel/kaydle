use std::{
    collections::HashMap,
    io::{self, Read},
};

use anyhow::Context;
use kaydle::serde::de::deserializer;
use kaydle_primitives::{string::KdlString, value::KdlValue};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename = "book")]
struct Book {
    title: String,
    author: String,
    year: i32,
}

fn main() -> anyhow::Result<()> {
    let mut buf = String::new();

    io::stdin()
        .read_to_string(&mut buf)
        .context("Failed to read from stdin")?;

    let values: Vec<Book> =
        Vec::deserialize(deserializer(&buf)).context("Failed to deserialize")?;

    println!("{:#?}", values);

    Ok(())
}
