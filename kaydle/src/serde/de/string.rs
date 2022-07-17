use std::marker::PhantomData;

use kaydle_primitives::string::KdlString;
use serde::{de, forward_to_deserialize_any};

use super::util::Unit;

// TODO: enum support.

#[derive(Debug)]
pub struct Deserializer<'i, E> {
    string: KdlString<'i>,
    error: PhantomData<E>,
}

impl<'i, E> Deserializer<'i, E> {
    pub fn new(string: KdlString<'i>) -> Self {
        Self {
            string,
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
        self.string.visit_to(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(self)
    }
}

impl<'de, E: de::Error> de::EnumAccess<'de> for Deserializer<'de, E> {
    type Error = E;
    type Variant = Unit<E>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self).map(|variant| (variant, Unit::new()))
    }
}
