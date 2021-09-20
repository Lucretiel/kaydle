use std::{char::CharTryFromError, num::ParseIntError};

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

#[derive(Debug)]
enum ProcessorState<'i, 'p> {
    Parent(&'p mut &'i str),
    Disconnected(&'i str),
}

use ProcessorState::*;

impl<'i, 'p> ProcessorState<'i, 'p> {
    fn get_input(&self) -> &'i str {
        match *self {
            Parent(&mut s) | Disconnected(s) => s,
        }
    }

    fn get_input_mut(&mut self) -> &mut &'i str {
        match self {
            Parent(s) => &mut **s,
            Disconnected(ref mut s) => s,
        }
    }

    fn run_parser<O, E>(&mut self, parser: impl Parser<&'i str, O, E>) -> Result<O, NomErr<E>> {
        run_parser_on(self.get_input_mut(), parser)
    }

    fn merge_into(self, original: &mut Self) {
        *original.get_input_mut() = self.get_input()
    }
}

impl<'i, 'p> Clone for ProcessorState<'i, 'p> {
    fn clone(&self) -> Self {
        Disconnected(self.get_input())
    }
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

/// Trait for types that can parse a node list. Abstracts over a node document
/// processor, which operates at the top level, and a node children processor,
/// which is nested in `{ }`
pub trait NodeListProcessor<'i, 'p>: Sized {
    /// Get the next node. Returns the node name and a processor
    fn next_node<'s, T, E>(&'s mut self) -> Result<Option<(T, NodeProcessor<'i, 's>)>, NomErr<E>>
    where
        T: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>;

    fn drain<E>(mut self) -> Result<(), NomErr<E>>
    where
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: FromExternalError<&'i str, BoundsError>,
        E: ContextError<&'i str>,
    {
        while let Some(((), node)) = self.next_node()? {
            node.drain()?;
        }

        Ok(())
    }
}

/// Processor for a top level kdl document.
#[derive(Debug, Clone)]
pub struct NodeDocumentProcessor<'i> {
    state: &'i str,
}

impl<'i> NodeDocumentProcessor<'i> {
    pub fn new(input: &'i str) -> Self {
        Self { state: input }
    }

    fn run_parser<T, E>(&mut self, parser: impl Parser<&'i str, T, E>) -> Result<T, NomErr<E>> {
        run_parser_on(&mut self.state, parser)
    }
}

impl<'i, 'p> NodeListProcessor<'i, 'p> for NodeDocumentProcessor<'i> {
    fn next_node<'s, T, E>(&'s mut self) -> Result<Option<(T, NodeProcessor<'i, 's>)>, NomErr<E>>
    where
        T: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>,
    {
        self.run_parser(parse_node_start(eof.value(())))
            .map(move |node_name| {
                node_name.map(move |node_name| {
                    (
                        node_name,
                        NodeProcessor {
                            state: ProcessorState::Parent(&mut self.state),
                        },
                    )
                })
            })
    }
}

enum InternalNodeEvent<V, K, P> {
    Value(V),
    Property(GenericProperty<K, P>),
    Children,
    End,
}

pub enum NodeEvent<'i, 'p, V, K, P> {
    Value(V, NodeProcessor<'i, 'p>),
    Property(GenericProperty<K, P>, NodeProcessor<'i, 'p>),
    Children(NodeChildrenProcessor<'i, 'p>),
    End,
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
pub struct NodeProcessor<'i, 'p> {
    state: ProcessorState<'i, 'p>,
}

impl<'i, 'p> NodeProcessor<'i, 'p> {
    pub fn merge_into(self, original: &mut Self) {
        self.state.merge_into(&mut original.state)
    }

    // Parse and discard everything in this node
    pub fn drain<E>(mut self) -> Result<(), NomErr<E>>
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
                NodeEvent::End => break Ok(()),
            }
        }
    }
}

impl<'i, 'p> NodeProcessor<'i, 'p> {
    pub fn next_event<V, K, P, E>(mut self) -> Result<NodeEvent<'i, 'p, V, K, P>, NomErr<E>>
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
        self.state
            .run_parser(parse_node_event)
            .map(move |event| match event {
                InternalNodeEvent::Value(value) => NodeEvent::Value(value, self),
                InternalNodeEvent::Property(prop) => NodeEvent::Property(prop, self),
                InternalNodeEvent::Children => {
                    NodeEvent::Children(NodeChildrenProcessor { state: self.state })
                }
                InternalNodeEvent::End => NodeEvent::End,
            })
    }
}

#[derive(Debug, Clone)]
pub struct NodeChildrenProcessor<'i, 'p> {
    state: ProcessorState<'i, 'p>,
}

impl<'i, 'p> NodeListProcessor<'i, 'p> for NodeChildrenProcessor<'i, 'p> {
    fn next_node<'s, T, E>(&'s mut self) -> Result<Option<(T, NodeProcessor<'i, 's>)>, NomErr<E>>
    where
        T: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>,
    {
        self.state
            .run_parser(parse_node_start(char('}')))
            .map(move |node_name| match node_name {
                Some(node_name) => Some((
                    node_name,
                    NodeProcessor {
                        state: ProcessorState::Parent(self.state.get_input_mut()),
                    },
                )),
                None => {
                    self.state = Disconnected("}");
                    None
                }
            })
    }
}

impl<'i, 'p> NodeListProcessor<'i, 'p> for &mut NodeChildrenProcessor<'i, 'p> {
    fn next_node<'s, T, E>(&'s mut self) -> Result<Option<(T, NodeProcessor<'i, 's>)>, NomErr<E>>
    where
        T: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, ParseIntError>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>,
    {
        NodeChildrenProcessor::next_node(self)
    }
}
