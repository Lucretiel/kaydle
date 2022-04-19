use kaydle_primitives::{
    annotation::{Annotated, AnnotatedValue, RecognizedAnnotationValue},
    node::{DrainOutcome, NodeContent, NodeEvent, NodeList},
    property::RecognizedProperty,
};
use serde::{de, Deserializer as _};

use super::{value::Deserializer as ValueDeserializer, Error};

#[derive(Debug)]
pub struct Deserializer<'i, 'p> {
    node: Annotated<'i, NodeContent<'i, 'p>>,
}

impl<'i, 'p> Deserializer<'i, 'p> {
    pub fn new(node: Annotated<'i, NodeContent<'i, 'p>>) -> Self {
        Self { node }
    }

    /// Deserialize a single primitive value, like a number, string, unit,
    /// etc. Doesn't apply to named data.
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

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // Need to visit Some or None. Need to also preserve annotation.
        // Probably need to do something sensible with children.
        todo!()
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
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::IncompatibleNode)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::IncompatibleNode)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
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

struct ArgumentSeqAccess<'i, 'p> {
    first: Option<AnnotatedValue<'i>>,
    node: NodeContent<'i, 'p>,
}

impl<'de> serde_mobile::SeqAccess<'de> for ArgumentSeqAccess<'de, '_> {
    type Error = Error;

    fn next_element_seed<S>(self, seed: S) -> Result<Option<(S::Value, Self)>, Self::Error>
    where
        S: de::DeserializeSeed<'de>,
    {
        let (value, tail) = match self.first {
            Some(value) => (value, self.node),
            None => match self.node.next_event()? {
                NodeEvent::Argument { argument, tail } => (argument, tail),
                NodeEvent::Property {
                    property: RecognizedProperty { .. },
                    tail,
                } => {
                    tail.drain()?;
                    return Err(Error::IncompatibleNode);
                }
                NodeEvent::Children { children } => {
                    return match children.drain()? {
                        DrainOutcome::Empty => Ok(None),
                        DrainOutcome::NotEmpty => Err(Error::IncompatibleNode),
                    }
                }
                NodeEvent::End => return Ok(None),
            },
        };

        seed.deserialize(ValueDeserializer::new(value))
            .map(|value| {
                (
                    value,
                    ArgumentSeqAccess {
                        first: None,
                        node: tail,
                    },
                )
            })
            .map(Some)
    }
}
