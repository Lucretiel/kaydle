use kaydle_primitives::{annotation::AnnotatedValue, string::KdlString, value::KdlValue};
use serde::{
    de::{self, value::BorrowedStrDeserializer},
    forward_to_deserialize_any,
};

use crate::serde::{
    de::{annotation::Deserializer as AnnotationDeserializer, Error},
    magics,
};

use super::raw;

#[derive(Debug, Clone)]
pub struct Deserializer<'a> {
    value: AnnotatedValue<'a>,
}

impl<'a> Deserializer<'a> {
    pub fn new(value: AnnotatedValue<'a>) -> Self {
        Self { value }
    }
}

// TODO: most of this implementation should forward directly to
// raw::Deserializer. Write a macro or trait to help with this
// forwarding.
impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        raw::Deserializer::new(self.value.item).deserialize_any(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map
        identifier  enum newtype_struct
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        raw::Deserializer::new(self.value.item).deserialize_option(visitor)
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
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
        match *fields {
            [magics::ANNOTATION, field_name] | [field_name, magics::ANNOTATION] => visitor
                .visit_map(serde_mobile::AccessAdapter::new(AnnotatedKeyAccess::new(
                    field_name, self.value,
                ))),
            _ if fields.contains(&magics::ANNOTATION) => Err(Error::InvalidAnnotatedValue),
            _ => raw::Deserializer::new(self.value.item).deserialize_struct(name, fields, visitor),
        }
    }
}

enum AnnotatedKeyAccess<'i> {
    Annotation {
        value: KdlValue<'i>,
        field_name: &'static str,
        annotation: Option<KdlString<'i>>,
    },
    Field {
        value: KdlValue<'i>,
        field_name: &'static str,
    },
}

enum AnnotatedValueAccess<'i> {
    Annotation {
        value: KdlValue<'i>,
        field_name: &'static str,
        annotation: Option<KdlString<'i>>,
    },
    Field {
        value: KdlValue<'i>,
    },
}

impl<'i> AnnotatedKeyAccess<'i> {
    pub fn new(field_name: &'static str, value: AnnotatedValue<'i>) -> Self {
        Self::Annotation {
            value: value.item,
            annotation: value.annotation,
            field_name,
        }
    }

    pub fn extract_key_value(self) -> (&'static str, AnnotatedValueAccess<'i>) {
        match self {
            AnnotatedKeyAccess::Annotation {
                value,
                field_name,
                annotation,
            } => (
                magics::ANNOTATION,
                AnnotatedValueAccess::Annotation {
                    value,
                    field_name,
                    annotation,
                },
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
            AnnotatedValueAccess::Annotation {
                value,
                field_name,
                annotation,
            } => seed
                .deserialize(AnnotationDeserializer::new(annotation))
                .map(|annotation| {
                    (
                        annotation,
                        Some(AnnotatedKeyAccess::Field { value, field_name }),
                    )
                }),

            AnnotatedValueAccess::Field { value } => seed
                .deserialize(raw::Deserializer::new(value))
                .map(|value| (value, None)),
        }
    }
}
