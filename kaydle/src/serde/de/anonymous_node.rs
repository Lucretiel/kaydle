use kaydle_primitives::{
    annotation::{Annotated, AnnotatedValue, RecognizedAnnotated, RecognizedAnnotationValue},
    node::{DrainOutcome, NodeContent, NodeEvent, NodeList},
    property::{Property, RecognizedProperty},
};
use serde::{de, Deserializer as _};
use serde_mobile::SubordinateValue;

use super::{
    node_list, string::Deserializer as StringDeserializer, util,
    value::annotated::Deserializer as ValueDeserializer, Error,
};

#[derive(Debug)]
pub struct Deserializer<'i, 'p> {
    node: Annotated<'i, NodeContent<'i, 'p>>,
}

impl<'i, 'p> Deserializer<'i, 'p> {
    pub fn new(node: Annotated<'i, NodeContent<'i, 'p>>) -> Self {
        Self { node }
    }

    /// Deserialize a single primitive value, like a number, string, unit,
    /// etc. Doesn't apply to named data. Absence of any value is handled as
    /// a unit. Generally, in order to work correctly,
    fn deserialize_primitive_value<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'i>,
    {
        match self.node.item.next_event()? {
            NodeEvent::Argument {
                argument: RecognizedAnnotationValue { item: value, .. },
                tail,
            } => match tail.drain()? {
                DrainOutcome::Empty => value.visit_to(visitor),
                DrainOutcome::NotEmpty => Err(Error::IncompatibleNode),
            },
            NodeEvent::Property {
                property: RecognizedProperty { .. },
                tail,
            } => {
                tail.drain()?;
                Err(Error::IncompatibleNode)
            }
            NodeEvent::Children { children } => match children.drain()? {
                DrainOutcome::Empty => visitor.visit_unit(),
                DrainOutcome::NotEmpty => Err(Error::IncompatibleNode),
            },
            NodeEvent::End => visitor.visit_unit(),
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de, '_> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::IncompatibleNode)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // Need to visit Some or None. Need to also preserve annotation.
        // Probably need to do something sensible with children.
        // General design thoughts: We can probably treat a node as either an
        // aggregate or a primitive (that is to say, we shouldn't worry about
        // the Option<Vec<i32>> case). This means that if we field a request
        // for an Option, we should probably treat it as an optional primitive
        todo!("Deserializing nodes into options is not implemented yet")
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
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
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // TODO: consider handling newtype structs as single element tuples?
        todo!(
            "Deserializing nodes into newtype structs isn't implemented yet \
            (did you mean to include #[serde(transparent)] on type {:?}?",
            name
        )
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // Basic logic: a sequence is either of arguments or children.
        match self.node.item.next_event()? {
            NodeEvent::Argument { argument, tail } => {
                let mut access = ArgumentsSeqAccess {
                    peeked: Some(argument),
                    node: Some(tail),
                };

                let value = visitor.visit_seq(&mut access)?;

                match access.peeked {
                    Some(..) => Err(Error::UnfinishedNode),
                    None => match access.node {
                        None => Ok(value),
                        Some(node) => match node.drain()? {
                            DrainOutcome::Empty => Ok(value),

                            // TODO: Need to inspect here to distinguish
                            // Unfinished from Incompatible
                            DrainOutcome::NotEmpty => Err(Error::IncompatibleNode),
                        },
                    },
                }
            }
            NodeEvent::Property {
                property: RecognizedProperty { .. },
                tail,
            } => {
                tail.drain()?;
                Err(Error::IncompatibleNode)
            }
            NodeEvent::Children { mut children } => {
                let value = visitor.visit_seq(node_list::SeqAccess::new(&mut children))?;
                match children.drain()? {
                    DrainOutcome::Empty => Ok(value),
                    DrainOutcome::NotEmpty => Err(Error::UnfinishedNode),
                }
            }
            NodeEvent::End => visitor.visit_seq(util::EmptyAccess::new()),
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
        match self.node.item.next_event()? {
            NodeEvent::Argument {
                argument: RecognizedAnnotated { .. },
                tail,
            } => {
                tail.drain()?;
                Err(Error::IncompatibleNode)
            }
            NodeEvent::Property { property, tail } => {
                use serde_mobile::AccessAdapter;

                let mut access = AccessAdapter::new(PropertiesMapAccess {
                    node: tail,
                    peeked: Some(property),
                });

                let value = visitor.visit_map(&mut access)?;

                // If empty is false, the node was definitely not fully
                // consumed. It still needs to be drained, but even if the
                // drain is empty it's already too late.
                let (empty, tail) = match access {
                    AccessAdapter::Done => return Ok(value),
                    AccessAdapter::Ready(PropertiesMapAccess { peeked, node }) => {
                        (peeked.is_none(), node)
                    }
                    AccessAdapter::Value(SubordinateValue {
                        parent: PropertiesMapAccess { node, .. },
                        ..
                    }) => (true, node),
                };

                match tail.drain()? {
                    DrainOutcome::Empty if empty => Ok(value),
                    _ => Err(Error::UnfinishedNode),
                }
            }
            NodeEvent::Children { mut children } => {
                let value = visitor.visit_map(node_list::MapAccess::new(&mut children))?;
                match children.drain()? {
                    DrainOutcome::Empty => Ok(value),
                    DrainOutcome::NotEmpty => Err(Error::UnfinishedNode),
                }
            }
            NodeEvent::End => visitor.visit_map(util::EmptyAccess::new()),
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
        if let Some(field) = fields.iter().find(|field| field.starts_with("$kaydle::")) {
            todo!(
                "kaydle magics aren't implemented yet; \
                found {:?} on on type {:?})",
                field,
                name
            );
        }
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!(
            "Deserializing anonymous nodes into enums isn't implemented yet \
            (did you mean to add #[serde(transparent)] to an enclosing \
            newtype struct?)"
        )
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive_value(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.node.item.drain()?;
        visitor.visit_unit()
    }
}

// VariantAccess for an anonymous node deseri
impl<'de> de::VariantAccess<'de> for Deserializer<'de, '_> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        de::Deserialize::deserialize(self)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_struct("-", fields, visitor)
    }
}

/// Type providing sequence access to the arguments of a node
// TODO: replace a struct containing options
// TODO: opt-in property / children caching.
struct ArgumentsSeqAccess<'i, 'a> {
    peeked: Option<AnnotatedValue<'i>>,
    node: Option<NodeContent<'i, 'a>>,
}

impl<'de, 'a> de::SeqAccess<'de> for ArgumentsSeqAccess<'de, 'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.peeked.take() {
            Some(argument) => seed.deserialize(ValueDeserializer::new(argument)).map(Some),
            None => match self.node.take() {
                None => Ok(None),
                Some(node) => match node.next_event()? {
                    NodeEvent::Argument { argument, tail } => {
                        self.node = Some(tail);
                        seed.deserialize(ValueDeserializer::new(argument)).map(Some)
                    }
                    NodeEvent::Property {
                        property: RecognizedProperty { .. },
                        tail,
                    } => {
                        self.node = Some(tail);

                        // This is where the buffering needs to happen
                        Err(Error::IncompatibleNode)
                    }
                    NodeEvent::Children { children } => match children.drain()? {
                        DrainOutcome::Empty => Ok(None),
                        DrainOutcome::NotEmpty => Err(Error::IncompatibleNode),
                    },
                    NodeEvent::End => Ok(None),
                },
            },
        }
    }
}

struct PropertiesMapAccess<'i, 'a> {
    peeked: Option<Property<'i>>,
    node: NodeContent<'i, 'a>,
}

impl<'de, 'a> serde_mobile::MapKeyAccess<'de> for PropertiesMapAccess<'de, 'a> {
    type Error = Error;
    type Value = serde_mobile::SubordinateValue<ValueDeserializer<'de>, Self>;

    fn next_key_seed<S>(self, seed: S) -> Result<Option<(S::Value, Self::Value)>, Self::Error>
    where
        S: de::DeserializeSeed<'de>,
    {
        let (property, tail) = match self.peeked {
            Some(property) => (property, self.node),
            None => match self.node.next_event()? {
                NodeEvent::Argument {
                    argument: RecognizedAnnotated { .. },
                    tail,
                } => {
                    // Need to buffer the argument here, then loop
                    tail.drain()?;
                    return Err(Error::IncompatibleNode);
                }
                NodeEvent::Property { property, tail } => (property, tail),
                NodeEvent::Children { children } => {
                    return match children.drain()? {
                        DrainOutcome::Empty => Ok(None),
                        DrainOutcome::NotEmpty => Err(Error::IncompatibleNode),
                    }
                }
                NodeEvent::End => return Ok(None),
            },
        };

        seed.deserialize(StringDeserializer::new(property.key))
            .map(|deserialized| {
                Some((
                    deserialized,
                    serde_mobile::SubordinateValue {
                        parent: PropertiesMapAccess {
                            peeked: None,
                            node: tail,
                        },
                        value: ValueDeserializer::new(property.value),
                    },
                ))
            })
    }
}
