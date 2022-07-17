use std::marker::PhantomData;

use serde::{de, forward_to_deserialize_any};
/// SeqAccess and MapAccess type that's always empty
pub struct EmptyAccess<E> {
    error: PhantomData<E>,
}

impl<E: de::Error> EmptyAccess<E> {
    pub fn new() -> Self {
        Self { error: PhantomData }
    }
}

impl<'de, E: de::Error> de::SeqAccess<'de> for EmptyAccess<E> {
    type Error = E;

    #[inline]
    fn next_element_seed<T>(&mut self, _seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        Ok(None)
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }
}

impl<'de, E: de::Error> de::MapAccess<'de> for EmptyAccess<E> {
    type Error = E;

    fn next_key_seed<K>(&mut self, _seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, _seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        panic!("called next_value_seed out of order")
    }

    fn next_entry_seed<K, V>(
        &mut self,
        _: K,
        _: V,
    ) -> Result<Option<(K::Value, V::Value)>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        Ok(None)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }
}

/// For things that are always units
pub struct Unit<E> {
    error: PhantomData<E>,
}

impl<E> Unit<E> {
    pub fn new() -> Self {
        Self { error: PhantomData }
    }
}

impl<'de, E: de::Error> de::Deserializer<'de> for Unit<E> {
    type Error = E;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any enum
    }
}

impl<'de, E: de::Error> de::VariantAccess<'de> for Unit<E> {
    type Error = E;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }
}
