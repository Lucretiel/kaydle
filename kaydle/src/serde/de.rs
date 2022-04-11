mod helpers;
mod list;
mod node;

use std::{
    fmt::Display,
    mem::{self},
};

use kaydle_primitives::{
    node::{
        NodeChildrenProcessor, NodeDocumentProcessor, NodeEvent, NodeListProcessor, NodeProcessor,
    },
    property::{Property, RecognizedProperty},
    string::KdlString,
    value::{KdlValue, RecognizedValue},
};
use nom::Err as NomErr;
use serde::{
    de::{
        self,
        value::{MapAccessDeserializer, SeqAccessDeserializer},
        IntoDeserializer, MapAccess, SeqAccess,
    },
    forward_to_deserialize_any,
};
use thiserror::Error;

use crate::serde::magic;
use helpers::{EmptyDeserializer, StringDeserializer, ValueDeserializer};

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

    #[error("wasn't expecting a node")]
    UnexpectedNode,

    #[error("wasn't expecting the node to end")]
    UnexpectedEndOfNode,

    #[error("attempted to parse a non-unit enum variant from a plain KDL value")]
    UnitVariantRequired,

    #[error("attempted to deserialize a node into a struct of the wrong name")]
    NodeNameMismatch,

    #[error("error")]
    Custom(String),
}

impl Error {
    fn from_parse_error(_err: NomErr<()>) -> Self {
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

enum NodeListSequenceState {
    /// We're interpreting this as a flat list:
    ///
    /// ```kdl
    /// item abc
    /// item def
    /// ghi
    /// ```
    ///
    /// We've picked out the node identifier. Nodes that don't use this
    /// identifier are in error.
    FlatList,

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
}

fn expect_node_completed<'i, 'p>(processor: NodeProcessor<'i, 'p>) -> Result<(), Error> {
    match processor.next_event().map_err(Error::from_parse_error)? {
        NodeEvent::Value((), ..) => Err(Error::UnexpectedValue),
        NodeEvent::Property(RecognizedProperty { .. }, ..) => Err(Error::UnexpectedProperty),
        NodeEvent::Children(..) => Err(Error::UnexpectedChildren),
        NodeEvent::End => Ok(()),
    }
}

fn expect_node_list_completed<'i, 'p>(
    mut processor: impl NodeListProcessor<'i, 'p>,
) -> Result<(), Error> {
    match processor.next_node().map_err(Error::from_parse_error)? {
        Some(((), ..)) => Err(Error::UnexpectedNode),
        None => Ok(()),
    }
}

trait NodeDeserializerStateReceiver {
    fn receive_flatlist(self);
    fn receive_stringlist(self) -> Result<(), Error>;
}

/// Deserializer for a single node where we haven't handled the node name yet.
/// The node name may be handled as a string, enum variant, or struct name.
struct NamedNodeDeserializer<'p, 'de, R: NodeDeserializerStateReceiver> {
    processor: NodeProcessor<'de, 'p>,
    state: R,
    node_name: KdlString<'de>,
}

impl<'p, 'de, R: NodeDeserializerStateReceiver> NamedNodeDeserializer<'p, 'de, R> {
    /// Deserialize a bool, int, etc. Doesn't handle strings or nulls, those
    /// are separate (because they can have a wider range of representations).
    /// This is specifically for cases where the state unconditionally becomes
    /// flatlist. This requires a single KDL value (not more than 1, and not
    /// a property or children)
    fn deserialize_primitive<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        self.state.receive_flatlist();

        match self
            .processor
            .next_event()
            .map_err(Error::from_parse_error)?
        {
            NodeEvent::Value(value, processor) => {
                expect_node_completed(processor)?;
                KdlValue::visit_to(value, visitor)
            }
            NodeEvent::Property(RecognizedProperty { .. }, ..) => Err(Error::UnexpectedProperty),
            NodeEvent::Children(..) => Err(Error::UnexpectedChildren),
            NodeEvent::End => visitor.visit_unit(),
        }
    }

    fn check_struct_name(
        self,
        name: &'static str,
    ) -> Result<BlankNodeDeserializer<'p, 'de>, Error> {
        if self.node_name == name {
            self.state.receive_flatlist();
            Ok(BlankNodeDeserializer {
                processor: self.processor,
            })
        } else {
            Err(Error::NodeNameMismatch)
        }
    }
}

impl<'de, R: NodeDeserializerStateReceiver> de::Deserializer<'de>
    for NamedNodeDeserializer<'_, 'de, R>
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // Not yet sure what to do here. Probably nothing, there's no way to
        // guess what a particular node interpretation might be.
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
                self.state.receive_flatlist();
                expect_node_completed(processor)?;
                KdlValue::visit_to(value, visitor)
            }
            NodeEvent::Property(RecognizedProperty { .. }, _processor) => {
                Err(Error::UnexpectedProperty)
            }
            NodeEvent::Children(_processor) => Err(Error::UnexpectedChildren),
            NodeEvent::End => {
                self.state.receive_stringlist()?;
                self.node_name.visit_to(visitor)
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
                self.state.receive_flatlist();

                match self
                    .processor
                    .next_event()
                    .map_err(Error::from_parse_error)?
                {
                    NodeEvent::Value((), processor) => expect_node_completed(processor)?,
                    NodeEvent::End => {}
                    NodeEvent::Property(RecognizedProperty { .. }, ..)
                    | NodeEvent::Children(..) => unreachable!(),
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
                self.state.receive_flatlist();
                KdlValue::visit_to(value, visitor)
            }
            NodeEvent::Property(RecognizedProperty { .. }, _processor) => {
                Err(Error::UnexpectedProperty)
            }
            NodeEvent::Children(_processor) => Err(Error::UnexpectedChildren),
            NodeEvent::End => {
                self.state.receive_flatlist();
                visitor.visit_unit()
            }
        }
    }

    fn deserialize_unit_struct<V>(
        mut self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.check_struct_name(name)?.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.check_struct_name(name)?;
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        *self.state = Some(NodeListSequenceState::FlatList);

        match self
            .processor
            .next_event()
            .map_err(Error::from_parse_error)?
        {
            NodeEvent::Value(value, processor) => {
                let mut values = ValuesSeqAccess {
                    first: Some(value),
                    processor: Some(processor),
                    skip_rule: UnexpectedIsError,
                };

                let result = visitor.visit_seq(&mut values)?;

                if let Some(processor) = values.processor {
                    expect_node_completed(processor)?;
                }

                Ok(result)
            }
            NodeEvent::Property(RecognizedProperty { .. }, _processor) => {
                Err(Error::UnexpectedProperty)
            }
            NodeEvent::Children(mut processor) => {
                let result = visitor.visit_seq(NodeListSeqAccess::new(&mut processor))?;
                expect_node_list_completed(processor)?;
                Ok(result)
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
        *self.state = Some(NodeListSequenceState::FlatList(self.node_name));

        match self
            .processor
            .next_event()
            .map_err(Error::from_parse_error)?
        {
            NodeEvent::Value((), ..) => Err(Error::UnexpectedValue),
            NodeEvent::Property(property, processor) => {
                let mut map = PropertiesMapAccess {
                    first_key: Some(property.key),
                    value: Some(property.value),
                    processor: Some(processor),
                    skip_rule: UnexpectedIsError,
                };

                let result = visitor.visit_map(&mut map)?;

                if let Some(processor) = map.processor {
                    expect_node_completed(processor)?;
                }

                Ok(result)
            }
            NodeEvent::Children(processor) => todo!(),
            NodeEvent::End => visitor.visit_map(EmptyDeserializer),
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
        *self.state = Some(NodeListSequenceState::FlatList(self.node_name));

        let mut collect_values = CollectRule::Done;
        let mut collect_properties = CollectRule::Dont;
        let mut collect_children = CollectRule::Dont;

        fn filter_noticed<'s>(
            slot: &'s mut CollectRule,
            name: &'static str,
        ) -> impl FnMut(&&str) -> bool + 's {
            move |field| {
                if *field == name {
                    *slot = CollectRule::Do;
                    false
                } else {
                    true
                }
            }
        }

        let fields = fields
            .iter()
            .copied()
            .filter(filter_noticed(&mut collect_values, "kdl::values"))
            .filter(filter_noticed(&mut collect_properties, "kdl::properties"))
            .filter(filter_noticed(&mut collect_children, "kdl::children"))
            .map(Some)
            .collect();

        let mut map = SimpleStructMapAccess {
            fields,
            collect_values,
            collect_properties,
            collect_children: false,
            state: MapAccessState::Key(self.processor),
        };

        visitor.visit_map(&mut map)
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
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.processor.drain().map_err(Error::from_parse_error)?;
        visitor.visit_unit()
    }
}

enum NextValue<'p, 'de> {
    // A value for a property
    Single(KdlValue<'de>, NodeProcessor<'de, 'p>),

    // The first in a series of values that need to be collected
    Value(KdlValue<'de>, NodeProcessor<'de, 'p>),

    // The first in a series of properties that need to be collected
    Property(Property<'de>, NodeProcessor<'de, 'p>),

    // The children that need to be collected
    Children(NodeChildrenProcessor<'de, 'p>),
}

enum MapAccessState<'p, 'de> {
    Key(NodeProcessor<'de, 'p>),
    Value(NextValue<'p, 'de>),
    Empty,
}

impl MapAccessState<'_, '_> {
    fn take(&mut self) -> Self {
        mem::replace(self, MapAccessState::Empty)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CollectRule {
    /// Don't collect entities of this type
    Dont,

    /// Do collect entities of this type
    Do,

    /// We've collected all the things and can skip these entities
    Done,
}

/// MapAccess type specifically for turning a node into a struct based on its
/// properties and values. Uses magic to check special cases.
struct SimpleStructMapAccess<'p, 'de> {
    // An ordered list of all the fields this struct is known to contain
    // (excluding kaydle magic fields). Fields are removed from this list as
    // they're deserialized
    fields: Vec<Option<&'static str>>,

    // Collect values into a single field called kdl::values
    collect_values: CollectRule,

    // Collect properties into a single field called kdl::properties
    collect_properties: CollectRule,

    // Collect children into a single field called kdl::children
    collect_children: bool,

    state: MapAccessState<'p, 'de>,
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

impl<'de> MapAccess<'de> for &mut SimpleStructMapAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        use CollectRule::*;

        match loop {
            match self.state.take() {
                MapAccessState::Empty => return Ok(None),
                MapAccessState::Value(..) => panic!("Called next_key_seed out of order"),
                MapAccessState::Key(processor) => {
                    break match (self.collect_properties, self.collect_values) {
                        (Done, Done) => {
                            match processor.next_event().map_err(Error::from_parse_error)? {
                                NodeEvent::Value((), processor)
                                | NodeEvent::Property(RecognizedProperty { .. }, processor) => {
                                    self.state = MapAccessState::Key(processor);
                                    continue;
                                }
                                NodeEvent::Children(processor) => {
                                    break NodeEvent::Children(processor)
                                }
                                NodeEvent::End => NodeEvent::End,
                            }
                        }
                        (Done, _) => {
                            match processor.next_event().map_err(Error::from_parse_error)? {
                                NodeEvent::Value(value, processor) => {
                                    NodeEvent::Value(value, processor)
                                }
                                NodeEvent::Property(RecognizedProperty { .. }, processor) => {
                                    self.state = MapAccessState::Key(processor);
                                    continue;
                                }
                                NodeEvent::Children(processor) => NodeEvent::Children(processor),
                                NodeEvent::End => NodeEvent::End,
                            }
                        }
                        (_, Done) => {
                            match processor.next_event().map_err(Error::from_parse_error)? {
                                NodeEvent::Value((), processor) => {
                                    self.state = MapAccessState::Key(processor);
                                    continue;
                                }
                                NodeEvent::Property(property, processor) => {
                                    NodeEvent::Property(property, processor)
                                }
                                NodeEvent::Children(processor) => NodeEvent::Children(processor),
                                NodeEvent::End => NodeEvent::End,
                            }
                        }
                        (_, _) => processor.next_event().map_err(Error::from_parse_error)?,
                    }
                }
            }
        } {
            NodeEvent::Value(value, processor) => match self.collect_values {
                Do => {
                    self.state = MapAccessState::Value(NextValue::Value(value, processor));
                    seed.deserialize(magic::NODE_VALUES_ID.into_deserializer())
                        .map(Some)
                }
                Dont => match self.take_next_unused_field() {
                    None => Err(Error::UnexpectedValue),
                    Some(key) => {
                        self.state = MapAccessState::Value(NextValue::Single(value, processor));
                        seed.deserialize(key.into_deserializer()).map(Some)
                    }
                },
                Done => unreachable!(),
            },
            NodeEvent::Property(property, processor) => match self.collect_properties {
                Do => {
                    self.state = MapAccessState::Value(NextValue::Property(property, processor));
                    seed.deserialize(magic::NODE_PROPERTIES_ID.into_deserializer())
                        .map(Some)
                }
                Dont => {
                    self.state =
                        MapAccessState::Value(NextValue::Single(property.value, processor));
                    self.take_field(&property.key);
                    seed.deserialize(StringDeserializer::new(property.key))
                        .map(Some)
                }

                Done => unreachable!(),
            },
            NodeEvent::Children(children) => {
                if self.collect_children {
                    self.state = MapAccessState::Value(NextValue::Children(children));
                    seed.deserialize(magic::NODE_CHILDREN_ID.into_deserializer())
                        .map(Some)
                } else {
                    Err(Error::UnexpectedChildren)
                }
            }
            NodeEvent::End => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match match self.state.take() {
            MapAccessState::Empty | MapAccessState::Key(..) => {
                panic!("called next_value_seed out of order")
            }
            MapAccessState::Value(next_value) => next_value,
        } {
            // Expecting a simple value as part of a single property
            NextValue::Single(value, processor) => {
                self.state = MapAccessState::Key(processor);
                seed.deserialize(ValueDeserializer::new(value))
            }

            // Expecting to deserialize a seq of values
            NextValue::Value(value, processor) => {
                let lookahead_processor = processor.clone();
                self.state = MapAccessState::Key(processor);
                self.collect_values = CollectRule::Done;

                // TODO: use a common re-shared buffer
                let mut seq = ValuesSeqAccess {
                    first: Some(value),
                    processor: Some(lookahead_processor),
                    skip_rule: UnexpectedPermissive,
                };

                let result = seed.deserialize(SeqAccessDeserializer::new(&mut seq));

                // Verify that the child deserializer consumed the whole node
                if let Some(processor) = seq.processor {
                    expect_node_completed(processor)?;
                }

                result
            }

            // Expecting to deserialize a map of properties
            NextValue::Property(property, processor) => {
                let lookahead_processor = processor.clone();
                self.state = MapAccessState::Key(processor);
                self.collect_properties = CollectRule::Done;

                // TODO: use a common re-shared buffer
                let mut seq = PropertiesMapAccess {
                    first_key: Some(property.key),
                    value: Some(property.value),
                    processor: Some(lookahead_processor),
                    skip_rule: UnexpectedPermissive,
                };

                let result = seed.deserialize(MapAccessDeserializer::new(&mut seq));

                // Verify that the child deserializer consumed the whole node
                if let Some(processor) = seq.processor {
                    expect_node_completed(processor)?;
                }

                result
            }

            // Expecting to deserialize a bunch of children.
            NextValue::Children(mut children) => {
                let result = seed.deserialize(NodeListDeserializer {
                    processor: &mut children,
                })?;

                match children.next_node().map_err(Error::from_parse_error)? {
                    None => Ok(result),
                    Some(((), _proc)) => Err(Error::UnexpectedNode),
                }
            }
        }
    }
}
