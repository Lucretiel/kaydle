mod annotation;
mod anonymous_node;
mod named_node;
mod node_list;
mod string;
mod util;
mod value;

use std::fmt::Debug;

use kaydle_primitives::node::Document;
use serde::de;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("error from Deserialize type: {0}")]
    Custom(String),

    #[error("can't deserialize primitive type from node list")]
    PrimitiveFromNodelist,

    #[error("parse error")]
    ParseError,

    #[error("a deserialize didn't use all the nodes in the list")]
    UnusedNode,

    #[error("attempted to deserialize a type called {type_name} from a node called {node_name}")]
    TypeNameMismatch {
        node_name: String,
        type_name: &'static str,
    },

    // TODO this is several different kinds of error
    #[error("attempted to deserialize a node, but it was incompatible")]
    IncompatibleNode,

    #[error("can't deserialize a primitive type from a named node")]
    PrimitiveFromNamedNode,

    #[error("got $kaydle::annotation, but the struct must have exactly two fields")]
    InvalidAnnotatedValue,

    #[error("tried to deserialize an enum from an annotated value. It must be a newtype enum.")]
    InvalidEnum,

    #[error("sequence or map deserializer didn't consume the whole node")]
    UnfinishedNode,
}

impl From<nom::Err<()>> for Error {
    fn from(_: nom::Err<()>) -> Self {
        Self::ParseError
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}

pub fn from_str<'a, T: de::Deserialize<'a>>(input: &'a str) -> Result<T, Error> {
    let document = Document::new(input);
    let deserializer = node_list::Deserializer::new(document);
    T::deserialize(deserializer)
}
