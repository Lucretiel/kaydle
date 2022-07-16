use kaydle_primitives::{
    annotation::{Annotated, GenericAnnotated},
    node::Node,
    string::KdlString,
};
use serde::de;

use super::{
    anonymous_node::Deserializer as AnonymousNodeDeserializer,
    string::Deserializer as StringDeserializer, Error,
};

#[derive(Debug)]
pub struct Deserializer<'i, 'p> {
    node: Annotated<'i, Node<'i, 'p, KdlString<'i>>>,
}

impl<'i, 'p> Deserializer<'i, 'p> {
    pub fn new(node: Annotated<'i, Node<'i, 'p, KdlString<'i>>>) -> Self {
        Self { node }
    }

    /// Extract the name from `self.node` and return the rest of it as an
    /// anonymous deserializer
    fn into_parts(self) -> (KdlString<'i>, AnonymousNodeDeserializer<'i, 'p>) {
        (
            self.node.item.name,
            AnonymousNodeDeserializer::new(GenericAnnotated {
                annotation: self.node.annotation,
                item: self.node.item.content,
            }),
        )
    }

    /// Treat this named node as an anonymous node if the node name is "-"
    fn become_anonymous(self) -> Result<AnonymousNodeDeserializer<'i, 'p>, Error> {
        let (name, node) = self.into_parts();
        (name == "-")
            .then(|| node)
            .ok_or(Error::PrimitiveFromNamedNode)
    }
}

macro_rules! anonymously {
    ($($deserialize:ident)*) => {
        $(
            fn $deserialize<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where V: de::Visitor<'de>,
            {
                self.become_anonymous()?.$deserialize(visitor)
            }
        )*
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

    anonymously! {
        deserialize_bool
        deserialize_i8 deserialize_i16 deserialize_i32 deserialize_i64
        deserialize_u8 deserialize_u16 deserialize_u32 deserialize_u64
        deserialize_f32 deserialize_f64 deserialize_char deserialize_str
        deserialize_string deserialize_bytes deserialize_byte_buf
        deserialize_option deserialize_unit deserialize_seq deserialize_map
        deserialize_identifier
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.become_anonymous()?.deserialize_tuple(len, visitor)
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let (node_name, node) = self.into_parts();

        match node_name == name {
            true => node.deserialize_unit(visitor),
            false => Err(Error::TypeNameMismatch {
                node_name: node_name.into_string(),
                type_name: name,
            }),
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let (node_name, node) = self.into_parts();

        // TODO: completion check here; need to ensure that node has
        // been drained.
        match node_name == name {
            true => visitor.visit_newtype_struct(node),
            false => Err(Error::TypeNameMismatch {
                node_name: node_name.into_string(),
                type_name: name,
            }),
        }
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
        let (node_name, node) = self.into_parts();

        match node_name == name {
            true => node.deserialize_tuple(len, visitor),
            false => Err(Error::TypeNameMismatch {
                node_name: node_name.into_string(),
                type_name: name,
            }),
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
        let (node_name, node) = self.into_parts();

        // TODO: various magics, and in particular a node name magic
        // TODO: add a deserialize_records method to Anonymous deserializer,
        // comparable to tuple vs tuple struct
        match node_name == name {
            true => node.deserialize_struct(name, fields, visitor),
            false => Err(Error::TypeNameMismatch {
                node_name: node_name.into_string(),
                type_name: name,
            }),
        }
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
        // TODO: check that self is drained afterwards. Probably want a
        // different type here.
        visitor.visit_enum(self)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.node.item.content.drain()?;
        visitor.visit_unit()
    }
}

impl<'de, 'p> de::EnumAccess<'de> for Deserializer<'de, 'p> {
    type Error = Error;
    type Variant = AnonymousNodeDeserializer<'de, 'p>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let (node_name, deserializer) = self.into_parts();

        seed.deserialize(StringDeserializer::new(node_name))
            .map(|variant| (variant, deserializer))
    }
}
