use kaydle_primitives::{
    node::{NodeEvent, NodeProcessor},
    property::RecognizedProperty,
    value::KdlValue,
};
use serde::de;

use crate::serde::de::{expect_node_completed, helpers::ValueDeserializer, Error};

use super::{Unexpected, UnexpectedIsError};

/// Deserialize a plain list of values into a sequence (like a Vec).
pub struct ValuesSeqAccess<'p, 'de, U: Unexpected<'p, 'de>> {
    first: Option<KdlValue<'de>>,
    processor: Option<NodeProcessor<'de, 'p>>,
    skip_rule: U,
}

impl<'p, 'de, U: Unexpected<'p, 'de>> ValuesSeqAccess<'p, 'de, U> {
    pub fn new(
        first: Option<KdlValue<'de>>,
        processor: NodeProcessor<'de, 'p>,
        skip_rule: U,
    ) -> Self {
        Self {
            first,
            processor: Some(processor),
            skip_rule,
        }
    }

    pub fn expect_node_completed(self) -> Result<(), Error> {
        match self.processor {
            Some(processor) => expect_node_completed(processor),
            None => Ok(()),
        }
    }
}

impl<'p, 'de> ValuesSeqAccess<'p, 'de, UnexpectedIsError> {
    /// Create a new ValuesSeqAccess where unrecognized components are treated
    /// as errors
    pub fn new_strict(first: Option<KdlValue<'de>>, processor: NodeProcessor<'de, 'p>) -> Self {
        Self::new(first, processor, UnexpectedIsError)
    }
}

impl<'p, 'de, U: Unexpected<'p, 'de>> de::SeqAccess<'de> for &mut ValuesSeqAccess<'p, 'de, U> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        loop {
            break match self.first.take() {
                Some(value) => seed.deserialize(ValueDeserializer::new(value)).map(Some),
                None => match self.processor.take() {
                    None => Ok(None),
                    Some(processor) => {
                        match processor.next_event().map_err(Error::from_parse_error)? {
                            NodeEvent::Value(value, processor) => {
                                self.processor = Some(processor);
                                seed.deserialize(ValueDeserializer::new(value)).map(Some)
                            }
                            NodeEvent::Property(RecognizedProperty { .. }, processor) => {
                                self.processor = Some(processor);
                                self.skip_rule.property()?;
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
}
