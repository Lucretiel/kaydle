use std::marker::PhantomData;

use derive_new::new;
use kaydle_primitives::value::KdlValue;
use serde::{de, forward_to_deserialize_any};

#[derive(Debug, Clone, new)]
pub struct Deserializer<'a, E> {
    value: KdlValue<'a>,
    error: PhantomData<E>,
}

impl<'de, E: de::Error> de::Deserializer<'de> for Deserializer<'de, E> {
    type Error = E;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.value.visit_to(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map
        identifier struct enum newtype_struct
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            KdlValue::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }
}
