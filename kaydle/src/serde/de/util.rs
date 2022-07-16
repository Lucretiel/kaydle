use std::marker::PhantomData;

use serde::de;
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
