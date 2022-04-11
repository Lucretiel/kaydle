use kaydle_primitives::node::{NodeListProcessor, NodeProcessor};
use serde::de;

use crate::serde::de::{helpers::StringDeserializer, node::deserializer::NodeDeserializer, Error};

/// Convert a node list into some kind of map. Each node name is a key of the
/// map, and the value is deserialized from the rest of the node
pub struct NodeListMapAccess<'p, 'de, T> {
    list_processor: &'p mut T,
    value_processor: Option<NodeProcessor<'de, 'p>>,
}

impl<'de, 'p, T> NodeListMapAccess<'p, 'de, T>
where
    T: NodeListProcessor<'de, 'p>,
{
    fn new(processor: &'p mut T) -> Self {
        Self {
            list_processor: processor,
            value_processor: None,
        }
    }
}

impl<'de, 'p, T> de::MapAccess<'de> for NodeListMapAccess<'p, 'de, T>
where
    T: NodeListProcessor<'de, 'p>,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let node = self
            .list_processor
            .next_node()
            .map_err(Error::from_parse_error)?;

        match node {
            None => Ok(None),
            Some((name, processor)) => {
                self.value_processor = Some(processor);
                seed.deserialize(StringDeserializer::new(name)).map(Some)
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(NodeDeserializer::new_nameless(
            self.value_processor
                .expect("called next_value_seed out of order"),
        ))
    }
}
