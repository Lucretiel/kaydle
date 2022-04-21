use kaydle_primitives::{
    annotation::{AnnotatedValue, GenericAnnotated, RecognizedAnnotationValue},
    string::KdlString,
    value::KdlValue,
};
use serde::{
    de::{self, IntoDeserializer},
    forward_to_deserialize_any,
};

use super::{string::Deserializer as StringDeserializer, Error};

#[derive(Debug)]
pub struct Deserializer<'i, A> {
    value: GenericAnnotated<A, KdlValue<'i>>,
}

impl<'i> Deserializer<'i, Option<KdlString<'i>>> {
    pub fn new(value: AnnotatedValue<'i>) -> Self {
        Self { value }
    }
}

impl<'i> Deserializer<'i, ()> {
    fn plain(value: KdlValue<'i>) -> Self {
        Self {
            value: GenericAnnotated {
                item: value,
                annotation: (),
            },
        }
    }
}

impl<'de, A> de::Deserializer<'de> for Deserializer<'de, A>
where
    GenericAnnotated<A, KdlValue<'de>>: HandleAnnotationMagic<'de>,
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.value.item.visit_to(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map
        identifier newtype_struct enum
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value.item {
            KdlValue::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.value.do_deserialize_struct(fields, visitor)
    }
}

pub trait HandleAnnotationMagic<'de> {
    fn do_deserialize_struct<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>;
}

impl<'de> HandleAnnotationMagic<'de> for AnnotatedValue<'de> {
    fn do_deserialize_struct<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match *fields {
            ["$kaydle::annotation", field_name] | [field_name, "$kaydle::annotation"] => visitor
                .visit_map(serde_mobile::AccessAdapter::new(AnnotatedKeyAccess::new(
                    field_name, self,
                ))),
            _ if fields.contains(&"$kaydle::annotation") => Err(Error::InvalidAnnotatedValue),
            _ => self.item.visit_to(visitor),
        }
    }
}

impl<'de> HandleAnnotationMagic<'de> for RecognizedAnnotationValue<'de> {
    fn do_deserialize_struct<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        self.item.visit_to(visitor)
    }
}

enum AnnotatedKeyAccess<'i> {
    Annotation {
        annotation: KdlString<'i>,
        value: KdlValue<'i>,
        field_name: &'static str,
    },
    Field {
        value: KdlValue<'i>,
        field_name: &'static str,
    },
}

enum AnnotatedValueAccess<'i> {
    Annotation {
        annotation: KdlString<'i>,
        value: KdlValue<'i>,
        field_name: &'static str,
    },
    Field {
        value: KdlValue<'i>,
    },
}

impl<'i> AnnotatedKeyAccess<'i> {
    pub fn new(field_name: &'static str, value: AnnotatedValue<'i>) -> Self {
        match value.annotation {
            Some(annotation) => Self::Annotation {
                annotation,
                value: value.item,
                field_name,
            },
            None => Self::Field {
                value: value.item,
                field_name,
            },
        }
    }
}

impl<'de> serde_mobile::MapKeyAccess<'de> for AnnotatedKeyAccess<'de> {
    type Error = Error;
    type Value = AnnotatedValueAccess<'de>;

    fn next_key_seed<S>(self, seed: S) -> Result<Option<(S::Value, Self::Value)>, Self::Error>
    where
        S: de::DeserializeSeed<'de>,
    {
        match self {
            AnnotatedKeyAccess::Annotation {
                annotation,
                value,
                field_name,
            } => seed
                .deserialize("$kaydle::annotation".into_deserializer())
                .map(|key| {
                    Some((
                        key,
                        AnnotatedValueAccess::Annotation {
                            annotation,
                            value,
                            field_name,
                        },
                    ))
                }),

            AnnotatedKeyAccess::Field { value, field_name } => seed
                .deserialize(field_name.into_deserializer())
                .map(|field_name| Some((field_name, AnnotatedValueAccess::Field { value }))),
        }
    }
}

impl<'de> serde_mobile::MapValueAccess<'de> for AnnotatedValueAccess<'de> {
    type Error = Error;
    type Key = AnnotatedKeyAccess<'de>;

    fn next_value_seed<S>(self, seed: S) -> Result<(S::Value, Option<Self::Key>), Self::Error>
    where
        S: de::DeserializeSeed<'de>,
    {
        match self {
            AnnotatedValueAccess::Annotation {
                annotation,
                value,
                field_name,
            } => seed
                .deserialize(StringDeserializer::new(annotation))
                .map(|annotation| {
                    (
                        annotation,
                        Some(AnnotatedKeyAccess::Field { value, field_name }),
                    )
                }),

            AnnotatedValueAccess::Field { value } => seed
                .deserialize(Deserializer::plain(value))
                .map(|value| (value, None)),
        }
    }
}
