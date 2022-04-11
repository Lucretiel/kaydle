use kaydle_primitives::{
    node::{NodeEvent, NodeProcessor},
    string::KdlString,
    value::KdlValue,
};
use serde::de;

use crate::serde::de::{
    helpers::{StringDeserializer, ValueDeserializer},
    Error,
};

use super::Unexpected;

/// Deserialize a plain list of properties into a map (like a HashMap). Other
/// events (values, children) are errors.
struct PropertiesMapAccess<'p, 'de, U: Unexpected<'p, 'de>> {
    first_key: Option<KdlString<'de>>,
    value: Option<KdlValue<'de>>,
    processor: Option<NodeProcessor<'de, 'p>>,
    skip_rule: U,
}

impl<'p, 'de, U: Unexpected<'p, 'de>> de::MapAccess<'de> for PropertiesMapAccess<'p, 'de, U> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        loop {
            break match self.first_key.take() {
                Some(key) => seed.deserialize(StringDeserializer::new(key)).map(Some),
                None => match self.processor.take() {
                    None => Ok(None),
                    Some(processor) => {
                        match processor.next_event().map_err(Error::from_parse_error)? {
                            NodeEvent::Property(property, processor) => {
                                self.processor = Some(processor);
                                self.value = Some(property.value);
                                seed.deserialize(StringDeserializer::new(property.key))
                                    .map(Some)
                            }
                            NodeEvent::Value((), processor) => {
                                self.processor = Some(processor);
                                self.skip_rule.value()?;
                                continue;
                            }
                            NodeEvent::Children(children) => {
                                self.skip_rule.children(children)?;
                                Ok(None)
                            }
                            NodeEvent::End => Ok(None),
                        }
                    }
                },
            };
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(ValueDeserializer::new(
            self.value
                .take()
                .expect("called next_value_seed out of order"),
        ))
    }
}
