use kaydle_primitives::node::NodeChildrenProcessor;

use crate::serde::de::Error;

pub mod map;
pub mod seq;
pub mod structured;

trait Unexpected<'p, 'de> {
    fn value(&mut self) -> Result<(), Error>;
    fn property(&mut self) -> Result<(), Error>;
    fn children(&mut self, children: NodeChildrenProcessor<'de, 'p>) -> Result<(), Error>;
}

struct UnexpectedIsError;

impl<'p, 'de> Unexpected<'p, 'de> for UnexpectedIsError {
    #[inline]
    fn value(&mut self) -> Result<(), Error> {
        Err(Error::UnexpectedValue)
    }

    #[inline]
    fn property(&mut self) -> Result<(), Error> {
        Err(Error::UnexpectedProperty)
    }

    #[inline]
    fn children(&mut self, _children: NodeChildrenProcessor<'de, 'p>) -> Result<(), Error> {
        Err(Error::UnexpectedChildren)
    }
}

struct UnexpectedPermissive;

impl<'p, 'de> Unexpected<'p, 'de> for UnexpectedPermissive {
    #[inline]
    fn value(&mut self) -> Result<(), Error> {
        Ok(())
    }
    #[inline]
    fn property(&mut self) -> Result<(), Error> {
        Ok(())
    }
    #[inline]
    fn children(&mut self, _children: NodeChildrenProcessor<'de, 'p>) -> Result<(), Error> {
        // In this case we don't consume children, on the assumption that this
        // is being used in a forked processor
        Ok(())
    }
}

struct UnexpectedPermissiveKeepChildren<'a, 'p, 'de> {
    slot: &'a mut Option<NodeChildrenProcessor<'de, 'p>>,
}

impl<'p, 'de> Unexpected<'p, 'de> for UnexpectedPermissiveKeepChildren<'_, 'p, 'de> {
    #[inline]
    fn value(&mut self) -> Result<(), Error> {
        Ok(())
    }
    #[inline]
    fn property(&mut self) -> Result<(), Error> {
        Ok(())
    }
    #[inline]
    fn children(&mut self, children: NodeChildrenProcessor<'de, 'p>) -> Result<(), Error> {
        if self.slot.replace(children).is_some() {
            panic!("Got into a bad state")
        }

        Ok(())
    }
}
