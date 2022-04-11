use kaydle_primitives::{
    node::{NodeEvent, NodeProcessor},
    property::RecognizedProperty,
    string::KdlString,
    value::{KdlValue, RecognizedValue},
};
use serde::de;

use crate::serde::de::{expect_node_completed, list::access::seq::NodeListSeqAccess, Error};

use super::access::seq::ValuesSeqAccess;

trait NodeNameAccess<'de> {
    fn take(&mut self) -> Option<KdlString<'de>>;
    fn check_struct_name(&self, name: &'static str) -> bool;
}

/// Node name access for anonymous nodes. Anonymous nodes mainly occur
/// when parsing a node list as a mapping, because the node name is
/// used as a key and is unavailable to the node as a value
struct Nameless;

impl<'de> NodeNameAccess<'de> for Nameless {
    fn take(&mut self) -> Option<KdlString<'de>> {
        None
    }

    fn check_struct_name(&self, name: &'static str) -> bool {
        true
    }
}

impl<'de> NodeNameAccess<'de> for Option<KdlString<'de>> {
    fn take(&mut self) -> Option<KdlString<'de>> {
        self.take()
    }

    fn check_struct_name(&self, name: &'static str) -> bool {
        match self {
            Some(string) => *string == name,
            None => true,
        }
    }
}

impl<'de, T: NodeNameAccess<'de>> NodeNameAccess<'de> for &mut T {
    fn take(&mut self) -> Option<KdlString<'de>> {
        T::take(self)
    }

    fn check_struct_name(&self, name: &'static str) -> bool {
        true
    }
}

pub struct NodeDeserializer<'p, 'de, N: NodeNameAccess<'de>> {
    processor: NodeProcessor<'de, 'p>,
    name: N,
}

impl<'p, 'de> NodeDeserializer<'p, 'de, Nameless> {
    pub fn new_nameless(processor: NodeProcessor<'de, 'p>) -> Self {
        Self {
            processor,
            name: Nameless,
        }
    }
}

impl<'p, 'de> NodeDeserializer<'p, 'de, Option<KdlString<'de>>> {
    pub fn new_named(processor: NodeProcessor<'de, 'p>, name: KdlString<'de>) -> Self {
        Self {
            processor,
            name: Some(name),
        }
    }
}

impl<'de, N: NodeNameAccess<'de>> NodeDeserializer<'_, 'de, N> {
    /// Deserialize a bool, int, etc.
    fn deserialize_primitive<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self
            .processor
            .next_event()
            .map_err(Error::from_parse_error)?
        {
            NodeEvent::Value(value, processor) => {
                expect_node_completed(processor)?;
                KdlValue::visit_to(value, visitor)
            }
            NodeEvent::Property(RecognizedProperty { .. }, ..) => Err(Error::UnexpectedProperty),
            NodeEvent::Children(..) => Err(Error::UnexpectedChildren),
            NodeEvent::End => visitor.visit_unit(),
        }
    }
}

impl<'p, 'de, N: NodeNameAccess<'de>> de::Deserializer<'de> for NodeDeserializer<'p, 'de, N> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // In all liklihood this will attempt to deserialize into a struct
        // using all kaydle magics
        todo!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let peek = self.processor.clone();

        match peek.next_event().map_err(Error::from_parse_error)? {
            NodeEvent::Value(RecognizedValue::Null, _) | NodeEvent::End => {
                match self
                    .processor
                    .next_event()
                    .map_err(Error::from_parse_error)?
                {
                    NodeEvent::Value((), processor) => expect_node_completed(processor)?,
                    NodeEvent::End => {}
                    NodeEvent::Property(RecognizedProperty { .. }, ..)
                    | NodeEvent::Children(..) => unreachable!(),
                }

                visitor.visit_none()
            }
            NodeEvent::Value(_, _)
            | NodeEvent::Property(RecognizedProperty { .. }, _)
            | NodeEvent::Children(_) => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_primitive(visitor)
    }

    fn deserialize_unit_struct<V>(
        mut self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.name.check_struct_name(name) {
            false => Err(Error::NodeNameMismatch),
            true => self.deserialize_primitive(visitor),
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
        let res = visitor.visit_newtype_struct(NodeDeserializer {
            processor: self.processor,
            name: &mut self.name,
        });

        match self.name.check_struct_name(name) {
            false => Err(Error::NodeNameMismatch),
            true => res,
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self
            .processor
            .next_event()
            .map_err(Error::from_parse_error)?
        {
            NodeEvent::Value(value, processor) => {
                let mut values = ValuesSeqAccess::new_strict(Some(value), processor);
                let result = visitor.visit_seq(&mut values)?;
                values.expect_node_completed()?;
                Ok(result)
            }
            NodeEvent::Property(RecognizedProperty { .. }, _processor) => {
                Err(Error::UnexpectedProperty)
            }
            NodeEvent::Children(mut processor) => {
                let result = visitor.visit_seq(NodeListSeqAccess::new(&mut processor))?;
                expect_node_list_completed(processor)?;
                Ok(result)
            }
            NodeEvent::End => visitor.visit_seq(EmptyDeserializer),
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

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        *self.state = Some(NodeListSequenceState::FlatList(self.node_name));

        match self
            .processor
            .next_event()
            .map_err(Error::from_parse_error)?
        {
            NodeEvent::Value((), ..) => Err(Error::UnexpectedValue),
            NodeEvent::Property(property, processor) => {
                let mut map = PropertiesMapAccess {
                    first_key: Some(property.key),
                    value: Some(property.value),
                    processor: Some(processor),
                    skip_rule: UnexpectedIsError,
                };

                let result = visitor.visit_map(&mut map)?;

                if let Some(processor) = map.processor {
                    expect_node_completed(processor)?;
                }

                Ok(result)
            }
            NodeEvent::Children(processor) => todo!(),
            NodeEvent::End => visitor.visit_map(EmptyDeserializer),
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
        *self.state = Some(NodeListSequenceState::FlatList(self.node_name));

        let mut collect_values = CollectRule::Done;
        let mut collect_properties = CollectRule::Dont;
        let mut collect_children = CollectRule::Dont;

        fn filter_noticed<'s>(
            slot: &'s mut CollectRule,
            name: &'static str,
        ) -> impl FnMut(&&str) -> bool + 's {
            move |field| {
                if *field == name {
                    *slot = CollectRule::Do;
                    false
                } else {
                    true
                }
            }
        }

        let fields = fields
            .iter()
            .copied()
            .filter(filter_noticed(&mut collect_values, "kdl::values"))
            .filter(filter_noticed(&mut collect_properties, "kdl::properties"))
            .filter(filter_noticed(&mut collect_children, "kdl::children"))
            .map(Some)
            .collect();

        let mut map = SimpleStructMapAccess {
            fields,
            collect_values,
            collect_properties,
            collect_children: false,
            state: MapAccessState::Key(self.processor),
        };

        visitor.visit_map(&mut map)
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
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.processor.drain().map_err(Error::from_parse_error)?;
        visitor.visit_unit()
    }
}
