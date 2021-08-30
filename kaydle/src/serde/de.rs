use std::{borrow::Cow, fmt::Display, mem::MaybeUninit, primitive};

use kaydle_primitives::{
    node::{
        NodeChildrenProcessor, NodeDocumentProcessor, NodeEvent, NodeListProcessor, NodeProcessor,
    },
    property::RecognizedProperty,
    string::KdlString,
    value::{GenericValue, KdlValue, RecognizedValue},
};
use nom::{combinator::eof, error::ParseError, Err as NomErr, Parser};
use nom_supreme::{final_parser::ExtractContext, ParserExt};
use serde::{de, Deserializer};
use thiserror::Error;

/// Deserializer for deserializing a Node list (such as a document or children).
/// Generally can only be used to deserialize sequences (maps, seqs, etc).
pub struct NodeListDeserializer<T> {
    processor: T,
}

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("there was an error trying to parse the input")]
    ParseError,

    #[error("attempted to deserialize a primitive from a node list")]
    AtNodeList,

    #[error("wasn't expecting a node property")]
    UnexpectedProperty,

    #[error("wasn't expecting node children")]
    UnexpectedChildren,

    #[error("wasn't expecting a value, or got too many values")]
    UnexpectedValue,

    #[error("wasn't expecting the node to end")]
    UnexpectedEndOfNode,

    #[error("error")]
    Custom(String),
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Custom(msg.to_string())
    }
}

impl<'de, 'p, T> de::Deserializer<'de> for NodeListDeserializer<T>
where
    T: NodeListProcessor<'de, 'p>,
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(NodeListSeqAccess {
            processor: self.processor,
            state: None,
        })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!();
    }
}

enum NodeListSequenceState<'de> {
    /// We're interpreting this as a flat list:
    ///
    /// item 10
    /// item 11
    /// item 12
    ///
    /// We've picked out the node identifier. Nodes that don't use this
    /// identifier are in error.
    FlatList(KdlString<'de>),

    /// We're interpreting this as a string list, where the node identifiers
    /// themselves are the values:
    ///
    /// Names {
    ///     name1
    ///     name2
    ///     name3
    /// }
    ///
    /// This might also be promoted to an enum list
    StringList,

    /// We're interpreting this as an enum list, where the node identifiers
    /// are enum discriminants:
    ///
    /// Names {
    ///     Nothing
    ///     Pair 1 2
    ///     Only 1
    /// }
    EnumList,
}

fn expect_node_completed<'i, 'p>(processor: NodeProcessor<'i, 'p>) -> Result<(), Error> {
    match processor
        .next_event()
        .map_err(|err: NomErr<()>| Error::ParseError)?
    {
        NodeEvent::Value((), _) => Err(Error::UnexpectedValue),
        NodeEvent::Property(RecognizedProperty { key: (), value: () }, _) => {
            Err(Error::UnexpectedProperty)
        }
        NodeEvent::Children(_) => Err(Error::UnexpectedChildren),
        NodeEvent::End => Ok(()),
    }
}

struct NodeListSeqAccess<'de, T> {
    processor: T,
    state: Option<NodeListSequenceState<'de>>,
}

impl<'de, 'p, T> de::SeqAccess<'de> for NodeListSeqAccess<'de, T>
where
    T: NodeListProcessor<'de, 'p>,
{
    type Error = Error;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>, Self::Error>
    where
        S: de::DeserializeSeed<'de>,
    {
        let node = self
            .processor
            .next_node()
            .map_err(|err: NomErr<()>| Error::ParseError)?;

        match node {
            Some((name, processor)) => match self.state {
                None => seed
                    .deserialize(BeginSeqNodeDeserializer {
                        name,
                        processor,
                        state: &mut self.state,
                    })
                    .map(Some),
                Some(state) => todo!(),
            },
            None => Ok(None),
        }
    }
}

/// Deserializer for a single node as part of a SeqAccess. This is the first
/// node in the sequence and is responsible for trying to detect the node list
/// pattern we're using.
struct BeginSeqNodeDeserializer<'p, 'de> {
    processor: NodeProcessor<'de, 'p>,
    state: &'p mut Option<NodeListSequenceState<'de>>,
    name: KdlString<'de>,
}

impl<'p, 'de> BeginSeqNodeDeserializer<'p, 'de> {
    /// Deserialize a bool, int, etc. Doesn't handle strings or nulls, those
    /// are separate (because they can have a wider range of representations).
    /// This is specifically for cases where the state unconditionally becomes
    /// flatlist. This requires a single KDL value (not more than 1, and not
    /// a property or children)
    fn deserialize_primitive<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        *self.state = Some(NodeListSequenceState::FlatList(self.name));

        match self
            .processor
            .next_event()
            .map_err(|err: NomErr<()>| Error::ParseError)?
        {
            NodeEvent::Value(value, processor) => {
                expect_node_completed(processor)?;
                KdlValue::visit_to(value, visitor)
            }
            NodeEvent::Property(RecognizedProperty { .. }, _processor) => {
                Err(Error::UnexpectedProperty)
            }
            NodeEvent::Children(_processor) => Err(Error::UnexpectedChildren),
            NodeEvent::End => Err(Error::UnexpectedEndOfNode),
        }
    }
}

impl<'de> de::Deserializer<'de> for BeginSeqNodeDeserializer<'_, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self
            .processor
            .next_event()
            .map_err(|err: NomErr<()>| Error::ParseError)?
        {
            NodeEvent::Value(value, processor) => {
                expect_node_completed(processor)?;
                *self.state = Some(NodeListSequenceState::FlatList(self.name));
                KdlValue::visit_to(value, visitor)
            }
            NodeEvent::Property(RecognizedProperty { .. }, _processor) => {
                Err(Error::UnexpectedProperty)
            }
            NodeEvent::Children(_processor) => Err(Error::UnexpectedChildren),
            NodeEvent::End => {
                *self.state = Some(NodeListSequenceState::StringList);
                self.name.visit_to(visitor)
            }
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // THIS IS WRONG: it needs expect_node_completed(processor)?;
        let peek = self.processor.clone();

        match peek
            .next_event()
            .map_err(|err: NomErr<()>| Error::ParseError)?
        {
            NodeEvent::Value(None, _) | NodeEvent::End => {
                *self.state = Some(NodeListSequenceState::FlatList(self.name));

                // extract the end-of-node or null from self
                let value: NodeEvent<(), (), ()> = self
                    .processor
                    .next_event()
                    .map_err(|err: NomErr<()>| Error::ParseError)?;

                visitor.visit_none()
            }
            NodeEvent::Value(Some(()), _)
            | NodeEvent::Property(RecognizedProperty { .. }, _)
            | NodeEvent::Children(_) => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(seq)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(data)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }
}

struct StackishVec<T, const N: usize> {
    size: usize,
    ptr: *mut T,
    local: [MaybeUninit<T>; N],
}

struct StackishVecValues<T> {
    capacity: usize,
    values: [T],
}
