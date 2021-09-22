use std::io::{self, Read};

use anyhow::Context;
use kaydle::serde::de::deserializer;
use serde::Deserialize;

fn main() -> anyhow::Result<()> {
    let mut buf = String::new();

    io::stdin()
        .read_to_string(&mut buf)
        .context("Failed to read from stdin")?;

    let values: Vec<i32> = Vec::deserialize(deserializer(&buf)).context("Failed to deserialize")?;

    println!("{:#?}", values);

    Ok(())
}
