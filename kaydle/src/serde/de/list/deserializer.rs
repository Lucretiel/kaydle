use kaydle_primitives::node::NodeListProcessor;
use serde::{de, forward_to_deserialize_any};

use crate::serde::de::{expect_node_list_completed, Error};

use super::access::seq::NodeListSeqAccess;

/// Deserializer for deserializing a Node list (such as a document or children).
/// Generally can only be used to deserialize sequences (maps, seqs, etc).
pub struct NodeListDeserializer<T> {
    processor: T,
}

impl<'de, 'p, T> de::Deserializer<'de> for NodeListDeserializer<T>
where
    T: NodeListProcessor<'de, 'p>,
{
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AtNodeList)
    }

    // Generally we can only parse structured types, like lists and maps, from
    // a node list. Primitives are no good.
    forward_to_deserialize_any! {
        bool
        i8 i16 i32 i64
        u8 u16 u32 u64
        f32 f64
        char str string bytes byte_buf
        option unit unit_struct
        enum identifier
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
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
        let value = visitor.visit_seq(NodeListSeqAccess::new(&mut self.processor))?;
        expect_node_list_completed(self.processor)?;
        Ok(value)
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
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    // Parse the entire node list
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.processor.drain().map_err(Error::from_parse_error)?;
        visitor.visit_unit()
    }
}
