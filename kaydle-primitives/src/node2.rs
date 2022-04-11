use std::{char::CharTryFromError, marker::PhantomData, num::ParseIntError};

use nom::{
    branch::alt,
    character::complete::char,
    combinator::eof,
    error::{ContextError, FromExternalError, ParseError},
    Err as NomErr, IResult, Parser,
};
use nom_supreme::{tag::TagError, ParserExt};

use crate::{
    number::BoundsError,
    property::{parse_property, GenericProperty, RecognizedProperty},
    string::{parse_identifier, StringBuilder},
    value::{parse_value, ValueBuilder},
    whitespace::{parse_linespace, parse_node_space, parse_node_terminator},
};

fn run_parser_on<I, O, E>(input: &mut I, mut parser: impl Parser<I, O, E>) -> Result<O, NomErr<E>>
where
    I: Clone,
{
    parser.parse(input.clone()).map(|(tail, value)| {
        *input = tail;
        value
    })
}

/// Parse the identifier at the start of a node, or some other subparser
/// indicating the end of a node list (either a } or an eof)
fn parse_node_start<'i, T, O, E>(
    end_of_nodes: impl Parser<&'i str, O, E>,
) -> impl Parser<&'i str, Option<T>, E>
where
    T: StringBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, ParseIntError>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
{
    parse_identifier
        .map(Some)
        .context("node")
        .or(end_of_nodes.map(|_| None))
        .preceded_by(parse_linespace)
}

#[derive(Debug, Clone)]
pub enum NextNode<'i, Name, Parent, Grandparent> {
    Node {
        name: Name,
        processor: NodeProcessor<'i, Parent>,
    },
    End {
        parent: Grandparent,
    },
}

/// Trait for types that can parse a node list. Abstracts over a node document
/// processor, which operates at the top level, and a node children processor,
/// which is nested in `{ }`
pub trait NodeListProcessor<'i>: Sized {
    type Parent;

    /// Create an instance of self. The input should be at a node list
    /// position.
    fn new(input: &'i str) -> Self;

    /// Get the next node. Returns the node name and a processor
    fn next_node<Name, E>(self) -> Result<NextNode<'i, Name, Self, Self::Parent>, NomErr<E>>
    where
        Name: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>;

    fn drain<E>(mut self) -> Result<Self::Parent, NomErr<E>>
    where
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: FromExternalError<&'i str, BoundsError>,
        E: ContextError<&'i str>,
    {
        loop {
            match self.next_node()? {
                NextNode::Node {
                    name: (),
                    processor,
                } => {
                    self = processor.drain()?;
                }
                NextNode::End { parent } => break Ok(parent),
            }
        }
    }
}

/// Processor for a top level kdl document.
#[derive(Debug, Clone)]
pub struct NodeDocumentProcessor<'i> {
    input: &'i str,
}

impl<'i> NodeDocumentProcessor<'i> {
    pub fn new(input: &'i str) -> Self {
        Self { input }
    }
}

impl<'i> NodeListProcessor<'i> for NodeDocumentProcessor<'i> {
    type Parent = ();

    fn new(input: &'i str) -> Self {
        NodeDocumentProcessor { input }
    }

    fn next_node<Name, E>(self) -> Result<NextNode<'i, Name, Self, ()>, NomErr<E>>
    where
        Name: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>,
    {
        parse_node_start(eof.value(()))
            .parse(self.input)
            .map(|(tail, node_name)| match node_name {
                None => NextNode::End { parent: () },
                Some(name) => NextNode::Node {
                    name,
                    processor: NodeProcessor {
                        input: tail,
                        parent: PhantomData,
                    },
                },
            })
    }
}

enum InternalNodeEvent<V, K, P> {
    Value(V),
    Property(GenericProperty<K, P>),
    Children,
    End,
}

pub enum NodeEvent<'i, V, K, P, T: NodeListProcessor<'i>> {
    Value(V, NodeProcessor<'i, T>),
    Property(GenericProperty<K, P>, NodeProcessor<'i, T>),
    Children(NodeChildrenProcessor<'i, T>),
    End(T),
}

fn parse_node_event<'i, E, V, K, P>(
    input: &'i str,
) -> IResult<&'i str, InternalNodeEvent<V, K, P>, E>
where
    V: ValueBuilder<'i>,
    K: StringBuilder<'i>,
    P: ValueBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, ParseIntError>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: FromExternalError<&'i str, BoundsError>,
    E: ContextError<&'i str>,
{
    alt((
        // Parse a value or property, preceded by 1 or more whitespace
        alt((
            // Important: make sure to try to parse a property first, since
            // "abc"=10 could be conservatively parsed as just the value "abc"
            // TODO: try to parse a value first, and if it's a string, try to
            // parse =value (in other words, avoid duplicating the string parse)
            parse_property
                .map(InternalNodeEvent::Property)
                .context("property"),
            parse_value.map(InternalNodeEvent::Value).context("value"),
        ))
        // Parse children or a node terminator, preceded by 0 or more whitespace
        .preceded_by(parse_node_space),
        alt((
            char('{')
                .map(|_| InternalNodeEvent::Children)
                .context("children"),
            parse_node_terminator.map(|()| InternalNodeEvent::End),
        ))
        .preceded_by(parse_node_space.opt()),
    ))
    .parse(input)
}

#[derive(Debug, Clone)]
pub struct NodeProcessor<'i, P> {
    input: &'i str,
    parent: PhantomData<P>,
}

impl<'i, T: NodeListProcessor<'i>> NodeProcessor<'i, T> {
    fn new(input: &'i str) -> Self {
        Self {
            input,
            parent: PhantomData,
        }
    }

    pub fn next_event<V, K, P, E>(mut self) -> Result<NodeEvent<'i, V, K, P, T>, NomErr<E>>
    where
        V: ValueBuilder<'i>,
        K: StringBuilder<'i>,
        P: ValueBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: FromExternalError<&'i str, BoundsError>,
        E: ContextError<&'i str>,
    {
        parse_node_event(self.input).map(|(tail, event)| match event {
            InternalNodeEvent::Value(value) => NodeEvent::Value(value, NodeProcessor::new(tail)),
            InternalNodeEvent::Property(prop) => {
                NodeEvent::Property(prop, NodeProcessor::new(tail))
            }
            InternalNodeEvent::Children => NodeEvent::Children(NodeChildrenProcessor::new(tail)),
            InternalNodeEvent::End => NodeEvent::End(T::new(tail)),
        })
    }

    // Parse and discard everything in this node
    pub fn drain<E>(mut self) -> Result<T, NomErr<E>>
    where
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: FromExternalError<&'i str, BoundsError>,
        E: ContextError<&'i str>,
    {
        loop {
            self = match self.next_event()? {
                NodeEvent::Value((), next) => next,
                NodeEvent::Property(RecognizedProperty { .. }, next) => next,
                NodeEvent::Children(children) => break children.drain(),
                NodeEvent::End(parent) => break Ok(parent),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeChildrenProcessor<'i, T: NodeListProcessor<'i>> {
    input: &'i str,
    parent: PhantomData<T>,
}

impl<'i, T: NodeListProcessor<'i>> NodeListProcessor<'i> for NodeChildrenProcessor<'i, T> {
    type Parent = T;

    fn new(input: &'i str) -> Self {
        Self {
            input,
            parent: PhantomData,
        }
    }

    fn next_node<Name, E>(self) -> Result<NextNode<'i, Name, Self, Self::Parent>, NomErr<E>>
    where
        Name: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>,
    {
        parse_node_start(char('}'))
            .parse(self.input)
            .map(|(tail, node_name)| match node_name {
                Some(name) => NextNode::Node {
                    name,
                    processor: NodeProcessor {
                        input: tail,
                        parent: PhantomData,
                    },
                },
                None => NextNode::End {
                    parent: T::new(tail),
                },
            })
    }
}
