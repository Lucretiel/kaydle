use kaydle_primitives::node::NodeListProcessor;
use serde::de;

use crate::serde::de::{node::deserializer::NodeDeserializer, Error};

/// Convert a node list into some kind of sequence. Each node is an item of the
/// sequence.
pub struct NodeListSeqAccess<'a, T> {
    processor: &'a mut T,
}

impl<'a, 'de, 'p, T> NodeListSeqAccess<'a, T>
where
    T: NodeListProcessor<'de, 'p>,
{
    pub fn new(processor: &'a mut T) -> Self {
        Self { processor }
    }
}

impl<'a, 'de, 'p, T> de::SeqAccess<'de> for NodeListSeqAccess<'a, T>
where
    T: NodeListProcessor<'de, 'p>,
{
    type Error = Error;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>, Self::Error>
    where
        S: de::DeserializeSeed<'de>,
    {
        let node = self
            .processor
            .next_node()
            .map_err(Error::from_parse_error)?;

        match node {
            None => Ok(None),
            Some((name, processor)) => seed
                .deserialize(NodeDeserializer::new_named(processor, name))
                .map(Some),
        }
    }
}
