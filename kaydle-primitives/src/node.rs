use std::char::CharTryFromError;

use nom::{
    branch::alt,
    character::complete::char,
    combinator::eof,
    error::{ContextError, FromExternalError, ParseError},
    Err as NomErr, IResult, Parser,
};
use nom_supreme::{tag::TagError, ParserExt};

use crate::{
    annotation::{with_annotation, AnnotationBuilder, GenericAnnotated, RecognizedAnnotation},
    number::BoundsError,
    property::{parse_property, GenericProperty, RecognizedProperty},
    string::{parse_identifier, StringBuilder},
    value::{parse_value, ValueBuilder},
    whitespace::{parse_linespace, parse_node_space, parse_node_terminator},
};

/// Run a parser on a mutable reference to some input. If the parse is
/// successful, the input is updated in-place, and the result of the parse
/// is returned.
fn run_parser_on<I, O, E>(input: &mut I, mut parser: impl Parser<I, O, E>) -> Result<O, NomErr<E>>
where
    I: Clone,
{
    parser.parse(input.clone()).map(|(tail, value)| {
        *input = tail;
        value
    })
}

/// Parse the annotation & identifier at the start of a node, or some other
/// subparser indicating the end of a node list (either a } or an eof)
fn parse_node_start<'i, T, A, E>(
    end_of_nodes: impl Parser<&'i str, (), E>,
) -> impl Parser<&'i str, Option<GenericAnnotated<A, T>>, E>
where
    T: StringBuilder<'i>,
    A: AnnotationBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
{
    with_annotation(parse_identifier)
        .map(Some)
        .context("node")
        .or(end_of_nodes.map(|()| None))
        .preceded_by(parse_linespace)
}

/// The return value of a successful fetch of the next node. Contains the
/// annotated name of the node as well as a node processor for fetching
/// arguments, properties, and children from the node.
pub struct NodeItem<'i, 'a, A, T> {
    pub name: GenericAnnotated<A, T>,
    pub content: NodeProcessor<'i, 'a>,
}

type EmptyNodeItem<'i, 'a> = NodeItem<'i, 'a, (), ()>;

/// Trait for types that can parse a node list. Abstracts over a node document
/// processor, which operates at the top level, and a node children processor,
/// which is nested in `{ }`
pub trait NodeListProcessor<'i, 'p>: Sized {
    /// Get the next node. Returns the node name and a processor.
    ///
    /// Note for implementors: this method should be fused, to ensure that
    /// `drain` is always safe to call. After it returns Ok(None), it should
    /// continue to return Ok(None) forever.
    fn next_node<'s, T, A, E>(&'s mut self) -> Result<Option<NodeItem<'i, 's, A, T>>, NomErr<E>>
    where
        T: StringBuilder<'i>,
        A: AnnotationBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>;

    /// Drain all remaining content from this nodelist. The nodelist is parsed,
    /// and errors are returned, but the content is otherwise discarded.
    fn drain<E>(mut self) -> Result<(), NomErr<E>>
    where
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,

        E: FromExternalError<&'i str, CharTryFromError>,
        E: FromExternalError<&'i str, BoundsError>,
        E: ContextError<&'i str>,
    {
        while let Some(EmptyNodeItem { content: node, .. }) = self.next_node()? {
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
    fn next_node<'s, T, A, E>(&'s mut self) -> Result<Option<NodeItem<'i, 's, A, T>>, NomErr<E>>
    where
        T: StringBuilder<'i>,
        A: AnnotationBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>,
    {
        self.run_parser(parse_node_start(eof.value(())))
            .map(move |node_name| {
                node_name.map(move |name| NodeItem {
                    name,
                    content: NodeProcessor {
                        state: &mut self.state,
                    },
                })
            })
    }
}

enum InternalNodeEvent<VA, V, K, PA, P> {
    Argument(GenericAnnotated<VA, V>),
    Property(GenericProperty<K, PA, P>),
    Children,
    End,
}

/// A piece of content from a node
#[derive(Debug)]
pub enum NodeEvent<'i, 'p, VA, V, K, PA, P> {
    /// An argument from a node
    Argument(
        /// The argument
        GenericAnnotated<VA, V>,
        /// The processor containing the rest of the node
        NodeProcessor<'i, 'p>,
    ),

    /// A property (key-value pair) from a node
    Property(
        /// The property
        GenericProperty<K, PA, P>,
        /// The processor containing the rest of the node
        NodeProcessor<'i, 'p>,
    ),

    /// A set of children from the node.
    Children(NodeChildrenProcessor<'i, 'p>),

    /// There was nothing else in the node.
    End,
}

fn parse_node_event<'i, E, VA, V, K, PA, P>(
    input: &'i str,
) -> IResult<&'i str, InternalNodeEvent<VA, V, K, PA, P>, E>
where
    V: ValueBuilder<'i>,
    VA: AnnotationBuilder<'i>,
    K: StringBuilder<'i>,
    P: ValueBuilder<'i>,
    PA: AnnotationBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
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
            parse_value
                .map(InternalNodeEvent::Argument)
                .context("value"),
        ))
        .preceded_by(parse_node_space),
        // Parse children or a node terminator, preceded by 0 or more whitespace
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

/// Type for retrieving the content (arguments, properties, and children) of
/// a single node. It's important to ensure you drain or otherwise consume all
/// events from this processor, or else the parent parser will be left in an
/// inconsistent state.
#[derive(Debug)]
pub struct NodeProcessor<'i, 'p> {
    state: &'p mut &'i str,
}

impl<'i, 'p> NodeProcessor<'i, 'p> {
    // Parse and discard everything in this node
    pub fn drain<E>(mut self) -> Result<(), NomErr<E>>
    where
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: FromExternalError<&'i str, BoundsError>,
        E: ContextError<&'i str>,
    {
        loop {
            self = match self.next_event()? {
                NodeEvent::Argument(RecognizedAnnotation { .. }, next) => next,
                NodeEvent::Property(RecognizedProperty { .. }, next) => next,
                NodeEvent::Children(children) => break children.drain(),
                NodeEvent::End => break Ok(()),
            }
        }
    }
}

impl<'i, 'p> NodeProcessor<'i, 'p> {
    pub fn next_event<VA, V, K, PA, P, E>(
        self,
    ) -> Result<NodeEvent<'i, 'p, VA, V, K, PA, P>, NomErr<E>>
    where
        V: ValueBuilder<'i>,
        VA: AnnotationBuilder<'i>,
        K: StringBuilder<'i>,
        P: ValueBuilder<'i>,
        PA: AnnotationBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: FromExternalError<&'i str, BoundsError>,
        E: ContextError<&'i str>,
    {
        run_parser_on(self.state, parse_node_event).map(move |event| match event {
            InternalNodeEvent::Argument(value) => NodeEvent::Argument(value, self),
            InternalNodeEvent::Property(prop) => NodeEvent::Property(prop, self),
            InternalNodeEvent::Children => NodeEvent::Children(NodeChildrenProcessor {
                state: Some(self.state),
            }),
            InternalNodeEvent::End => NodeEvent::End,
        })
    }
}

#[derive(Debug)]
pub struct NodeChildrenProcessor<'i, 'p> {
    state: Option<&'p mut &'i str>,
}

impl<'i, 'p> NodeListProcessor<'i, 'p> for NodeChildrenProcessor<'i, 'p> {
    fn next_node<'s, T, A, E>(&'s mut self) -> Result<Option<NodeItem<'i, 's, A, T>>, nom::Err<E>>
    where
        T: StringBuilder<'i>,
        A: AnnotationBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,

        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>,
    {
        match self.state.take() {
            None => Ok(None),
            Some(state) => match run_parser_on(state, parse_node_start(char('}').value(())))? {
                None => Ok(None),
                Some(name) => Ok(Some(NodeItem {
                    name,
                    content: NodeProcessor {
                        state: *self.state.insert(state),
                    },
                })),
            },
        }
    }
}

impl<'i, 'p> NodeListProcessor<'i, 'p> for &mut NodeChildrenProcessor<'i, 'p> {
    fn next_node<'s, T, A, E>(&'s mut self) -> Result<Option<NodeItem<'i, 's, A, T>>, NomErr<E>>
    where
        T: StringBuilder<'i>,
        A: AnnotationBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str>,
    {
        NodeChildrenProcessor::next_node(self)
    }
}

/// This test may not look like much, but all the relevant components are
/// separately tested. If `.drain()` works, it's very likely the entire
/// processor does too
#[test]
fn test_full_document_drain() {
    let content = r##"
    // This is a KDL document!
    node1 "arg1" prop=10 {
        (u8)item 10
        (u8)item 20
        items {
            a /* An important note here */ "abc"
            d "def"; g "ghi"
        }
    }
    (annotated)node2
    primitives null false true 10 10.5 -10 -10.5 3e6 0x10c 0b00001111 0o755
    (a)annotated (n)null (f)false (t)true (i)10 (f)10.5 (n)-10.5e7
    "##;

    let processor = NodeDocumentProcessor::new(content);

    let res: Result<(), nom::Err<()>> = processor.drain();
    res.expect("parse error");
}
