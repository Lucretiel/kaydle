use kaydle_primitives::{
    annotation::{AnnotatedValue, GenericAnnotated, RecognizedAnnotationValue},
    string::KdlString,
    value::KdlValue,
};
use serde::{
    de::{self, value::BorrowedStrDeserializer},
    forward_to_deserialize_any,
};

// TODO: I don't like the use of a single type to handle both raw values and
// annotated values; refactor.
// TODO: Annotations as enum discriminants

use super::{super::magics, annotation::Deserializer as AnnotationDeserializer, Error};

/// Deserializer for a KDL value with an optional annotation. Deserializes into
/// primitive types, but also can deserialize an annotation.
#[derive(Debug, Clone)]
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
        identifier enum newtype_struct
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
            [magics::ANNOTATION, field_name] | [field_name, magics::ANNOTATION] => visitor
                .visit_map(serde_mobile::AccessAdapter::new(AnnotatedKeyAccess::new(
                    field_name, self,
                ))),
            _ if fields.contains(&magics::ANNOTATION) => Err(Error::InvalidAnnotatedValue),
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
        value: AnnotatedValue<'i>,
        field_name: &'static str,
    },
    Field {
        value: KdlValue<'i>,
        field_name: &'static str,
    },
}

enum AnnotatedValueAccess<'i> {
    Annotation {
        value: AnnotatedValue<'i>,
        field_name: &'static str,
    },
    Field {
        value: KdlValue<'i>,
    },
}

impl<'i> AnnotatedKeyAccess<'i> {
    pub fn new(field_name: &'static str, value: AnnotatedValue<'i>) -> Self {
        Self::Annotation { value, field_name }
    }

    fn extract_key_value(self) -> (&'static str, AnnotatedValueAccess<'i>) {
        match self {
            AnnotatedKeyAccess::Annotation { value, field_name } => (
                magics::ANNOTATION,
                AnnotatedValueAccess::Annotation { value, field_name },
            ),
            AnnotatedKeyAccess::Field { value, field_name } => {
                (field_name, AnnotatedValueAccess::Field { value })
            }
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
        let (key, value) = self.extract_key_value();
        seed.deserialize(BorrowedStrDeserializer::new(key))
            .map(|key| Some((key, value)))
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
            AnnotatedValueAccess::Annotation { value, field_name } => seed
                .deserialize(AnnotationDeserializer::new(value.annotation))
                .map(|annotation| {
                    (
                        annotation,
                        Some(AnnotatedKeyAccess::Field {
                            value: value.item,
                            field_name,
                        }),
                    )
                }),

            AnnotatedValueAccess::Field { value } => seed
                .deserialize(Deserializer::plain(value))
                .map(|value| (value, None)),
        }
    }
}
