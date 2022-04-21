use std::{char::CharTryFromError, marker::PhantomData, ptr::NonNull};

use kaydle_primitives::{
    annotation::{Annotated, GenericAnnotated},
    node::{DrainOutcome, NodeContent, NodeList},
    string::StringBuilder,
};
use nom::error::{FromExternalError, ParseError};
use nom_supreme::{context::ContextError, tag::TagError};
use serde::{de, forward_to_deserialize_any};

use super::{
    anonymous_node::Deserializer as AnonymousNodeDeserializer,
    named_node::Deserializer as NamedNodeDeserializer, string::Deserializer as StringDeserializer,
    Error,
};

pub struct Deserializer<T> {
    list: T,
}

impl<'de, T: NodeList<'de>> de::Deserializer<'de> for Deserializer<T> {
    type Error = Error;

    fn deserialize_any<V>(self, _v: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::PrimitiveFromNodelist)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct enum identifier
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // TODO: recursion limit
        // TODO: Nested Errors
        let value = visitor.visit_seq(SeqAccess::new(&mut self.list))?;

        match self.list.drain()? {
            DrainOutcome::Empty => Ok(value),
            DrainOutcome::NotEmpty => Err(Error::UnusedNode),
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

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let value = visitor.visit_map(NodeListReborrower::new(&mut self.list))?;

        match self.list.drain()? {
            DrainOutcome::Empty => Ok(value),
            DrainOutcome::NotEmpty => Err(Error::UnusedNode),
        }
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

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.list.drain()?;
        visitor.visit_unit()
    }
}

pub struct SeqAccess<'a, L> {
    list: &'a mut L,
}

impl<'a, L> SeqAccess<'a, L> {
    pub fn new(list: &'a mut L) -> Self {
        Self { list }
    }
}

impl<'de, L> de::SeqAccess<'de> for SeqAccess<'_, L>
where
    L: NodeList<'de>,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.list
            .next_node()?
            .map(|node| seed.deserialize(NamedNodeDeserializer::new(node)))
            .transpose()
    }
}

// Please for the love of god someone tell me there's a different way to do this
struct NodeListReborrower<'i, 'a, T: NodeList<'i> + 'a> {
    fake_list: PhantomData<&'a mut T>,
    list: NonNull<T>,
    node: Option<Annotated<'i, NodeContent<'i, 'a>>>,
}

#[derive(Debug)]
enum NodeListReborrowError<E> {
    /// the reborrower currently has a node
    Borrowed,

    /// next_node returned an error
    Error(E),
}

enum NodeListUseNodeError {
    /// There wasn't a node available
    NoNode,
}

impl<'a, 'i, T: NodeList<'i>> NodeListReborrower<'i, 'a, T> {
    fn new(list: &'a mut T) -> Self {
        Self {
            fake_list: PhantomData,
            list: NonNull::from(list),
            node: None,
        }
    }

    fn next_node<E, N>(&mut self) -> Result<Option<N>, NodeListReborrowError<nom::Err<E>>>
    where
        N: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,

        E: ContextError<&'i str, &'static str>,
    {
        if self.node.is_some() {
            return Err(NodeListReborrowError::Borrowed);
        }

        // Invariant preserved: it's only safe for &'a mut T to exist if we
        // don't also own a NodeContent<'_, 'a>
        let list: &'a mut T = unsafe { self.list.as_mut() };
        let node = match list.next_node() {
            Err(err) => return Err(NodeListReborrowError::Error(err)),
            Ok(None) => return Ok(None),
            Ok(Some(node)) => node,
        };

        self.node = Some(GenericAnnotated {
            annotation: node.annotation,
            item: node.item.content,
        });

        Ok(Some(node.item.name))
    }

    fn use_node<R>(
        &mut self,
        op: impl for<'b> FnOnce(Annotated<'i, NodeContent<'i, 'b>>) -> R,
    ) -> Result<R, NodeListUseNodeError> {
        self.node.take().map(op).ok_or(NodeListUseNodeError::NoNode)
    }
}

impl<'de, T> de::MapAccess<'de> for NodeListReborrower<'de, '_, T>
where
    T: NodeList<'de>,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.next_node()
            .map_err(|err| match err {
                NodeListReborrowError::Borrowed => panic!("called next_key_seed out of order"),
                NodeListReborrowError::Error(err) => err,
            })?
            .map(|node_name| seed.deserialize(StringDeserializer::new(node_name)))
            .transpose()
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.use_node(|node| seed.deserialize(AnonymousNodeDeserializer::new(node)))
            .unwrap_or_else(|_| panic!("called next_value_seed out of order"))
    }
}
