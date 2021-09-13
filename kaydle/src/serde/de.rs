use std::{
    borrow::Cow,
    fmt::Display,
    mem::{self, MaybeUninit},
    primitive,
};

use kaydle_primitives::{
    node::{
        NodeChildrenProcessor, NodeDocumentProcessor, NodeEvent, NodeListProcessor, NodeProcessor,
    },
    property::{GenericProperty, Property, RecognizedProperty},
    string::{KdlString, StringBuilder},
    value::{GenericValue, KdlValue, RecognizedValue, ValueBuilder},
};
use nom::{combinator::eof, error::ParseError, Err as NomErr, Parser};
use nom_supreme::{final_parser::ExtractContext, ParserExt};
use serde::{
    de::{self, value::SeqAccessDeserializer, IntoDeserializer, MapAccess, SeqAccess},
    forward_to_deserialize_any, Deserializer,
};
use thiserror::Error;

use crate::serde::magic;

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

impl Error {
    fn from_parse_error(err: NomErr<()>) -> Self {
        Self::ParseError
    }
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
        visitor.visit_seq(NodeListSeqAccess::new(self.processor))
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
    /// ```kdl
    /// item 10
    /// item 11
    /// item 12
    /// ```
    ///
    /// We've picked out the node identifier. Nodes that don't use this
    /// identifier are in error.
    FlatList(KdlString<'de>),

    /// We're interpreting this as a string list, where the node identifiers
    /// themselves are the values:
    ///
    /// ```kdl
    /// Names {
    ///     name1
    ///     name2
    ///     name3
    /// }
    /// ```
    ///
    /// This might also be promoted to an enum list
    StringList,

    /// We're interpreting this as an enum list, where the node identifiers
    /// are enum discriminants:
    ///
    /// ```kdl
    /// Names {
    ///     Nothing
    ///     Pair 1 2
    ///     Only 1
    /// }
    /// ```
    EnumList,
}

fn expect_node_completed<'i, 'p>(processor: NodeProcessor<'i, 'p>) -> Result<(), Error> {
    match processor
        .next_recognized_event()
        .map_err(Error::from_parse_error)?
    {
        NodeEvent::Value(..) => Err(Error::UnexpectedValue),
        NodeEvent::Property(..) => Err(Error::UnexpectedProperty),
        NodeEvent::Children(..) => Err(Error::UnexpectedChildren),
        NodeEvent::End => Ok(()),
    }
}

struct NodeListSeqAccess<'de, T> {
    processor: T,
    state: Option<NodeListSequenceState<'de>>,
}

impl<'de, 'p, T> NodeListSeqAccess<'de, T>
where
    T: NodeListProcessor<'de, 'p>,
{
    fn new(processor: T) -> Self {
        Self {
            processor,
            state: None,
        }
    }
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
            .map_err(Error::from_parse_error)?;

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
            .map_err(Error::from_parse_error)?
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
            .map_err(Error::from_parse_error)?
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
        let peek = self.processor.clone();

        match peek.next_event().map_err(Error::from_parse_error)? {
            NodeEvent::Value(RecognizedValue::Null, _) | NodeEvent::End => {
                *self.state = Some(NodeListSequenceState::FlatList(self.name));

                match self
                    .processor
                    .next_event()
                    .map_err(Error::from_parse_error)?
                {
                    NodeEvent::Value((), processor) => expect_node_completed(processor)?,
                    NodeEvent::Property(RecognizedProperty { .. }, ..)
                    | NodeEvent::Children(..) => unreachable!(),
                    NodeEvent::End => {}
                }

                visitor.visit_none()
            }
            NodeEvent::Value(_, _)
            | NodeEvent::Property(RecognizedProperty { .. }, _)
            | NodeEvent::Children(_) => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self
            .processor
            .next_event()
            .map_err(Error::from_parse_error)?
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
                *self.state = Some(NodeListSequenceState::FlatList(self.name));
                visitor.visit_unit()
            }
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
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
        *self.state = Some(NodeListSequenceState::FlatList(self.name));

        match self
            .processor
            .next_event()
            .map_err(Error::from_parse_error)?
        {
            NodeEvent::Value(value, processor) => {
                let mut processor = Some(self.processor);

                let result = visitor.visit_seq(ValuesSeqAccess {
                    first: Some(value),
                    processor: &mut processor,
                })?;

                if let Some(processor) = processor {
                    expect_node_completed(processor)?;
                }

                Ok(result)
            }
            NodeEvent::Property(RecognizedProperty { .. }, processor) => {
                Err(Error::UnexpectedProperty)
            }
            NodeEvent::Children(..) => {
                let event: NodeEvent<(), (), ()> = self
                    .processor
                    .next_event()
                    .map_err(|err: NomErr<()>| Error::ParseError)?;

                match event {
                    NodeEvent::Children(processor) => {
                        visitor.visit_seq(NodeListSeqAccess::new(processor))
                    }
                    NodeEvent::Value(..) | NodeEvent::Property(..) | NodeEvent::End => {
                        unreachable!()
                    }
                }
            }
            NodeEvent::End => visitor.visit_seq(EmptyDeserializer),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        *self.state = Some(NodeListSequenceState::FlatList(self.name));

        match self
            .processor
            .next_event()
            .map_err(Error::from_parse_error)?
        {
            NodeEvent::Value((), processor) => Err(Error::UnexpectedValue),
            NodeEvent::Property(property, processor) => {
                let mut processor = Some(self.processor);

                let result = visitor.visit_map(PropertiesMapAccess {
                    key: Some(property.key),
                    value: Some(property.value),
                    processor: &mut processor,
                })?;

                if let Some(processor) = processor {
                    expect_node_completed(processor)?;
                }

                Ok(result)
            }
            NodeEvent::Children(..) => {
                let event: NodeEvent<(), (), ()> = self
                    .processor
                    .next_event()
                    .map_err(|err: NomErr<()>| Error::ParseError)?;

                match event {
                    NodeEvent::Children(processor) => {
                        visitor.visit_seq(NodeListSeqAccess::new(processor))
                    }
                    NodeEvent::Value(..) | NodeEvent::Property(..) | NodeEvent::End => {
                        unreachable!()
                    }
                }
            }
            NodeEvent::End => visitor.visit_seq(EmptyDeserializer),
        }
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

struct ValuesSeqAccess<'p, 'de> {
    first: Option<KdlValue<'de>>,
    processor: &'p Option<NodeProcessor<'de, 'p>>,
}

impl<'p, 'de> SeqAccess<'de> for ValuesSeqAccess<'p, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.first.take() {
            Some(value) => seed.deserialize(ValueDeserializer { value }).map(Some),
            None => match self.processor.take() {
                None => Ok(None),
                Some(mut processor) => {
                    match processor.next_event().map_err(Error::from_parse_error)? {
                        NodeEvent::Value(value, processor) => {
                            *self.processor = Some(processor);
                            seed.deserialize(ValueDeserializer { value }).map(Some)
                        }
                        NodeEvent::Property(RecognizedProperty { .. }, ..) => {
                            Err(Error::UnexpectedProperty)
                        }
                        NodeEvent::Children(processor) => Err(Error::UnexpectedChildren),
                        NodeEvent::End => Ok(None),
                    }
                }
            },
        }
    }
}

struct PropertiesMapAccess<'p, 'de> {
    key: Option<KdlString<'de>>,
    value: Option<KdlValue<'de>>,
    processor: &'p Option<NodeProcessor<'de, 'p>>,
}

impl<'de> MapAccess<'de> for PropertiesMapAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.key.take() {
            Some(key) => seed
                .deserialize(StringDeserializer { value: key })
                .map(Some),
            None => match self.processor.take() {
                None => Ok(None),
                Some(processor) => match processor.next_event().map_err(Error::from_parse_error)? {
                    NodeEvent::Value((), ..) => Err(Error::UnexpectedValue),
                    NodeEvent::Property(property, processor) => {
                        *self.processor = Some(processor);
                        self.value = Some(property.value);
                        seed.deserialize(StringDeserializer {
                            value: property.key,
                        })
                        .map(Some)
                    }
                    NodeEvent::Children(..) => Err(Error::UnexpectedChildren),
                    NodeEvent::End => Ok(None),
                },
            },
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(ValueDeserializer {
            value: self.value.expect("called next_value_seed out of order"),
        })
    }
}

enum NextValue<'p, 'de> {
    // A value for a property
    Value(KdlValue<'de>),

    // The first in a series of values that need to be collected
    Argument(KdlValue<'de>),

    // The first in a series of properties that need to be collected
    Property(Property<'de>),

    // The children that need to be collected
    Children(NodeChildrenProcessor<'de, 'p>),
}

/// MapAccess type specifically for turning a node into a struct based on its
/// properties and values. Uses magic to check special cases.
struct SimpleStructMapAccess<'p, 'de> {
    fields: Vec<Option<&'static str>>,
    collect_values: bool,
    collect_properties: bool,
    collect_children: bool,
    next_value: Option<NextValue<'p, 'de>>,
    processor: &'p mut Option<NodeProcessor<'de, 'p>>,
}

impl SimpleStructMapAccess<'_, '_> {
    fn take_next_unused_field(&mut self) -> Option<&'static str> {
        self.fields.iter_mut().find_map(|field| field.take())
    }

    fn take_field(&mut self, target: &str) {
        if let Some(field) = self.fields.iter_mut().find(|field| **field == Some(target)) {
            *field = None
        }
    }
}

impl<'de> MapAccess<'de> for SimpleStructMapAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.processor.take() {
            None => Ok(None),
            Some(processor) => match processor.next_event().map_err(Error::from_parse_error)? {
                NodeEvent::Value(value, processor) => {
                    *self.processor = Some(processor);

                    if self.collect_values {
                        self.next_value = Some(NextValue::Argument(value));
                        seed.deserialize(magic::NODE_VALUES_ID.into_deserializer())
                            .map(Some)
                    } else {
                        match self.take_next_unused_field() {
                            None => Err(Error::UnexpectedValue),
                            Some(key) => {
                                self.next_value = Some(NextValue::Value(value));
                                seed.deserialize(key.into_deserializer()).map(Some)
                            }
                        }
                    }
                }
                NodeEvent::Property(property, processor) => {
                    *self.processor = Some(processor);

                    if self.collect_properties {
                        self.next_value = Some(NextValue::Property(property));
                        seed.deserialize(magic::NODE_PROPERTIES_ID.into_deserializer())
                            .map(Some)
                    } else {
                        self.next_value = Some(NextValue::Value(property.value));
                        self.take_field(&property.key);
                        seed.deserialize(StringDeserializer {
                            value: property.key,
                        })
                        .map(Some)
                    }
                }
                NodeEvent::Children(children) => {
                    if self.collect_children {
                        self.next_value = Some(NextValue::Children(children));
                        seed.deserialize(magic::NODE_CHILDREN_ID.into_deserializer())
                            .map(Some)
                    } else {
                        Err(Error::UnexpectedChildren)
                    }
                }
                NodeEvent::End => Ok(None),
            },
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self
            .next_value
            .expect("called next_value_seed out of order")
        {
            // Expecting a simple value as part of a single property
            NextValue::Value(value) => seed.deserialize(ValueDeserializer { value }),

            // Expecting to deserialize a seq of values
            NextValue::Argument(value) => {
                // TODO: use a common re-shared buffer
                let mut properties = PropertiesBuffer::Empty;
                let mut children = None;
                let mut processor = self.processor.take();

                let result =
                    seed.deserialize(SeqAccessDeserializer::new(SimpleStructValuesCollector {
                        first: Some(value),
                        processor: &mut processor,
                        properties_buffer: &mut properties,
                        children_buffer: &mut children,
                    }));

                todo!("handle cleanup")
            }

            // Expecting to deserialize a map of properties
            NextValue::Property(_) => todo!(),

            // Expecting to deserialize a bunch of children. Use a
            // NodeListDeserializer here. Make sure to fork `children` so that
            // you can ensure that *all* nodes in the children set are consumed
            NextValue::Children(children) => {}
        }
    }
}

enum PropertiesBuffer<'de> {
    Empty,
    Property(Property<'de>),
    Properties(Vec<Property<'de>>),
}

struct SimpleStructValuesCollector<'p, 'de> {
    first: Option<KdlValue<'de>>,
    processor: &'p mut Option<NodeProcessor<'de, 'p>>,
    properties_buffer: &'p mut PropertiesBuffer<'de>,
    children_buffer: &'p mut Option<NodeChildrenProcessor<'de, 'p>>,
}

impl<'p, 'de> SeqAccess<'de> for SimpleStructValuesCollector<'p, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.first {
            Some(value) => seed.deserialize(ValueDeserializer { value }).map(Some),
            None => match self.processor.take() {
                None => Ok(None),
                Some(processor) => match processor.next_event().map_err(Error::from_parse_error)? {
                    NodeEvent::Value(value, processor) => {
                        *self.processor = Some(processor);
                        seed.deserialize(ValueDeserializer { value }).map(Some)
                    }
                    NodeEvent::Property(property, processor) => {
                        *self.processor = Some(processor);
                        match self.properties_buffer {
                            // If we haven't started a buffer, look ahead to see
                            // if we need to start one
                            PropertiesBuffer::Empty => {
                                *self.properties_buffer = PropertiesBuffer::Property(property);
                                todo!("Look ahead; are there any other values?")
                            }
                            // We definitely need to start a buffer in these
                            // cases, so there's no need to look ahead here
                            PropertiesBuffer::Property(_) => todo!(),
                            PropertiesBuffer::Properties(_) => todo!(),
                        }
                    }
                    NodeEvent::Children(children) => {
                        *self.children_buffer = Some(children);
                        Ok(None)
                    }
                    NodeEvent::End => Ok(None),
                },
            },
        }
    }
}

struct ValueDeserializer<'a> {
    value: KdlValue<'a>,
}

impl<'de> Deserializer<'de> for ValueDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.value.visit_to(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct StringDeserializer<'a> {
    value: KdlString<'a>,
}

impl<'de> Deserializer<'de> for StringDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.value.visit_to(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct EmptyDeserializer;

impl<'de> Deserializer<'de> for EmptyDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de> SeqAccess<'de> for EmptyDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        Ok(None)
    }
}

impl<'de> MapAccess<'de> for EmptyDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        panic!("No values in an EmptyDeserializer")
    }
}
