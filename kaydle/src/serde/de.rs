/*!
Serde Deserializer for KDL

# Example

```rust
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Eq)]
struct AnnotatedNumber {
    #[serde(rename="$kaydle::annotation")]
    annotation: Option<String>,
    value: i32,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
enum IntOrString {
    #[serde(rename="int")]
    Int(i32),

    #[serde(rename="string")]
    String(String),
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
struct IntString {
    int: i32,
    string: String,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
struct Document {
    name: String,
    number: i32,
    strings: Vec<String>,
    annotated: Vec<AnnotatedNumber>,
    annotated_enums: Vec<IntOrString>,
    key_value: HashMap<char, i32>,
    enum_list: Vec<IntOrString>,
    properties: IntString,
    children: IntString,
}

let document: Document = kaydle::serde::from_str(r#"
    name "Kat"
    number 10
    strings "A" "B" "C"

    // Values can include an annotation. Normally the annotation is ignored,
    // but you can use a struct containing a field called $kaydle::annotation
    // to include it in deserialization
    annotated 1 (abc)2 (def)3

    // Annotations can also be used as enum discriminants
    annotated_enums (int)10 (string)"hello"

    // If a mapping type is deserialized (like a struct or HashMap), node names
    // are used as keys
    key_value {
        a 1
        b 2
        c 3
    }

    // If a list of enums is deserialized, node names are used as enum
    // variants
    enum_list {
        int 10
        string "hello"
    }

    // kaydle treats properties and children similarly when deserializing maps.
    // in the future it will be possible to specify which one you wanted, or
    // use both.
    properties int=10 string="world"
    children {
        int 10
        string "world"
    }
"#).expect("failed to deserialize");

assert_eq!(
    document,
    Document {
        name: "Kat".to_owned(),
        number: 10,
        strings: Vec::from([
            "A".to_owned(),
            "B".to_owned(),
            "C".to_owned(),
        ]),
        annotated: Vec::from([
            AnnotatedNumber {
                annotation: None,
                value: 1,
            },
            AnnotatedNumber {
                annotation: Some("abc".to_owned()),
                value: 2,
            },
            AnnotatedNumber {
                annotation: Some("def".to_owned()),
                value: 3,
            },
        ]),
        annotated_enums: Vec::from([
            IntOrString::Int(10),
            IntOrString::String("hello".to_owned()),
        ]),
        key_value: HashMap::from([
            ('a', 1),
            ('b', 2),
            ('c', 3),
        ]),
        enum_list: Vec::from([
            IntOrString::Int(10),
            IntOrString::String("hello".to_owned()),
        ]),
        properties: IntString {
            int: 10,
            string: "world".to_owned(),
        },
        children: IntString {
            int: 10,
            string: "world".to_owned(),
        },
    },
);
```

# How it works

Check out the kaydle [specification](https://github.com/Lucretiel/kaydle/blob/main/SPEC.md)
for full details on how kaydle maps KDL to the serde data model. This section
is a brief summary, and in particular tries to document what is and isn't
implemented yet during these alpha releases.

At a high level, kaydle treats KDL content as being made of up 3 different
data structures:

- A **node list** is a list of nodes- either a Document or Children. It maps to
the serde data model as either a **sequence** of **named nodes** or a **mapping**
of strings to **anonymous nodes**
- A **named node** is a node that still has its name associated with it. Usually
named nodes appear when deserializing a **node list** as a sequence. Kaydle
required that named nodes "use" their name in some way- either the node name
must match the name of the deserialized type, or (if it's an enum) it's used
as the enum discriminant. A node name of "-" is treated the same as an
**anonymous node**
- An **anonymous node** is a node that doesn't have its name associated with it.
This can happen because the node list is being deserialized as a mapping (so
the name was used as a key), or because the name was used as an enum
discriminant. **Anonymous nodes** can be treated as (most) primitive values,
in which case they must contain a single argument and nothing else. They can
alternatively be treated as sequences or maps, in which case the node must
contain *only* arguments *or* properties *or* children (in the future it will
be possible to use specially named struct fields to extract nodes with more
than one of these).

A KDL value maps directly to the serde data model in the ways you might expect
(strings, booleans, null, strings, etc). Annotations are ignored by default,
but they can be used in two places:
- You can deserialize an annotated value into a struct containing a field called
`$kaydle::annotation` to retrieve the value annotation. The struct should
contain one additional field to store the value itself.
- You can deserialize an annotated value into a newtype enum, in which case the
annotation is used as an enum discriminant.

# Unimplemented limitations

- The major unimplemented thing at this point are most kaydle magics:
`$kaydle::children`, `$kaydle::arguments`, etc. These will allow nodes
containing mixes of children, arguments, and properties to be deserialized.
- Anonymous nodes cannot yet be deserialized into enums. This will be handled
by using the first argument as a discriminant.
- Anonymous nodes cannot yet be deserialized into options.
*/

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

/// Deserialization errors
#[derive(Debug, Error)]
pub enum Error {
    /// There was an error from a `Deserialize` type
    #[error("error from Deserialize type: {0}")]
    Custom(String),

    /// Attempted to turn a Document or Children into a primitive value, like
    /// an int or string
    #[error("can't deserialize primitive type from node list")]
    PrimitiveFromNodelist,

    /// There was a parse error
    #[error("parse error")]
    ParseError,

    /// Didn't consume all the nodes from a document or children
    #[error("a deserialize didn't use all the nodes in the list")]
    UnusedNode,

    /// The node name didn't match the newtype name
    #[error("attempted to deserialize a type called {type_name} from a node called {node_name}")]
    TypeNameMismatch {
        /// The name of the node being deserialized
        node_name: String,

        /// The name of the type being deserialized, which should match the node_name
        type_name: &'static str,
    },

    /// The node wasn't compatible with the type being deserialized (eg, tried
    /// to deserialize a HashMap from a node containing arguments instead of
    /// properties)
    #[error("attempted to deserialize a node, but it was incompatible")]
    IncompatibleNode,

    /// Attempted to deserialize a primitive type from a named node (use a
    /// newtype or struct wrapper instead)
    #[error("can't deserialize a primitive type from a named node")]
    PrimitiveFromNamedNode,

    /// The deserialize type included a field called `$kaydle::annotation`; such
    /// types must have exactly one additional field.
    #[error("got $kaydle::annotation, but the struct must have exactly two fields")]
    InvalidAnnotatedValue,

    /// The Deserialize type didn't consume the entire node
    #[error("sequence or map deserializer didn't consume the whole node")]
    UnfinishedNode,

    /// A non-newtype enum was deserialized from an annotated value
    #[error("only newtype variants can be deserialized from `(annotation)value` values")]
    NonNewtypeFromAnnotatedValue,
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

/// Deserialize something from a string containing a KDL document.
/// See [module][crate::serde::de] docs for details
pub fn from_str<'a, T: de::Deserialize<'a>>(input: &'a str) -> Result<T, Error> {
    let document = Document::new(input);
    let deserializer = node_list::Deserializer::new(document);
    T::deserialize(deserializer)
}

pub use node_list::Deserializer;
