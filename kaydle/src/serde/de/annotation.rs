use std::marker::PhantomData;

use kaydle_primitives::string::KdlString;
use serde::{de, forward_to_deserialize_any};

use super::string::Deserializer as StringDeserializer;

/// An Annotation Deserializer. Operates on an `Option<String>`, but also
/// accepts requests to deserialize as a string directly. May also learn how
/// to deserialize into unit enum variants.

#[derive(Debug, Clone)]
pub struct Deserializer<'i, E> {
    annotation: Option<KdlString<'i>>,
    error: PhantomData<E>,
}

impl<'i, E> Deserializer<'i, E> {
    pub fn new(annotation: Option<KdlString<'i>>) -> Self {
        Self {
            annotation,
            error: PhantomData,
        }
    }
}

impl<'de, E: de::Error> de::Deserializer<'de> for Deserializer<'de, E> {
    type Error = E;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.annotation {
            Some(annotation) => visitor.visit_some(StringDeserializer::new(annotation)),
            None => visitor.visit_none(),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char
        bytes byte_buf option newtype_struct seq tuple
        tuple_struct map struct ignored_any
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.annotation {
            Some(annotation) => annotation.visit_to(visitor),
            None => visitor.visit_none(),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.annotation {
            Some(annotation) => visitor.visit_some(StringDeserializer::new(annotation)),
            None => visitor.visit_unit(),
        }
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

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.annotation {
            Some(annotation) => {
                StringDeserializer::new(annotation).deserialize_enum(name, variants, visitor)
            }
            None => visitor.visit_none(),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }
}
