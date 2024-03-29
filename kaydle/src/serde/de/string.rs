use std::marker::PhantomData;

use derive_new::new;
use kaydle_primitives::string::KdlString;
use serde::{de, forward_to_deserialize_any};

// TODO: enum support.

#[derive(Debug, new)]
pub struct Deserializer<'i, E> {
    string: KdlString<'i>,
    error: PhantomData<E>,
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
        tuple_struct map struct enum identifier ignored_any
    }
}
