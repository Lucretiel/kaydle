pub mod anonymous_node;
pub mod named_node;
pub mod node_list;
pub mod string;

use std::fmt::Debug;

use serde::de;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("error from Deserialize type: {0}")]
    Custom(String),

    #[error("can't deserialize primitive type from node list")]
    NodelistFromPrimitive,

    #[error("parse error")]
    ParseError,

    #[error("a deserialize didn't use all the nodes in the list")]
    UnusedNode,

    #[error("attempted to deserialize a type called {type_name} from a node called {node_name}")]
    TypeNameMismatch {
        node_name: String,
        type_name: &'static str,
    },
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
