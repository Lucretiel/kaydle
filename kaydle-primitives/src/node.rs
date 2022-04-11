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
    annotation::{with_annotation, AnnotationBuilder, GenericAnnotated},
    number::BoundsError,
    property::{parse_property, GenericProperty},
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
    pub node: NodeProcessor<'i, 'a>,
}

type RecognizedNodeItem<'i, 'a> = NodeItem<'i, 'a, (), ()>;

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
        while let Some(RecognizedNodeItem { node, .. }) = self.next_node()? {
            node.drain()?;
        }

        Ok(())
    }
}

/// Processor for a top level kdl document.
#[derive(Debug, Clone)]
pub struct NodeDocumentProcessor<'i> {
    /// The original input string
    state: &'i str,

    /// Bool that ensures that node processors fully consume their nodes, so
    /// that parse state remains consistent. Set to true when a node processor
    /// is returned, and only resets to false when that processor is finished.
    node_in_progress: bool,
}

impl<'i> NodeDocumentProcessor<'i> {
    pub fn new(input: &'i str) -> Self {
        Self {
            state: input,
            node_in_progress: false,
        }
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
        if self.node_in_progress {
            panic!(
                "Called next_node before the previous node was fully parsed. This is a kaydle bug."
            )
        }

        self.run_parser(parse_node_start(eof.value(())))
            .map(move |node_name| {
                node_name.map(move |name| {
                    self.node_in_progress = true;

                    NodeItem {
                        name,
                        node: NodeProcessor {
                            state: &mut self.state,
                            in_progress: &mut self.node_in_progress,
                        },
                    }
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
    Argument {
        /// The argument
        argument: GenericAnnotated<VA, V>,
        /// The processor containing the rest of the node
        tail: NodeProcessor<'i, 'p>,
    },

    /// A property (key-value pair) from a node
    Property {
        /// The property
        property: GenericProperty<K, PA, P>,
        /// The processor containing the rest of the node
        tail: NodeProcessor<'i, 'p>,
    },

    /// A set of children from the node.
    Children {
        children: NodeChildrenProcessor<'i, 'p>,
    },

    /// There was nothing else in the node.
    End,
}

/// A [`NodeEvent`] containing no data. Used when the caller care what _kind_ of
/// thing was in the node, but not the actual value / content.
pub type RecognizedNodeEvent<'i, 'p> = NodeEvent<'i, 'p, (), (), (), (), ()>;

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

    /// Bool owned by the parent's list processor. Must be set to false only when
    /// this node has been fully consumed.
    in_progress: &'p mut bool,
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
                RecognizedNodeEvent::Argument { tail, .. } => tail,
                RecognizedNodeEvent::Property { tail, .. } => tail,
                RecognizedNodeEvent::Children { children } => break children.drain(),
                RecognizedNodeEvent::End => break Ok(()),
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
        // Because we use a move-oriented interface, there's no need to check
        // in_progress. We (or the children processor we return) just need to
        // make sure it's reset to false when we're done.
        run_parser_on(self.state, parse_node_event).map(move |event| match event {
            InternalNodeEvent::Argument(argument) => NodeEvent::Argument {
                argument,
                tail: self,
            },
            InternalNodeEvent::Property(property) => NodeEvent::Property {
                property,
                tail: self,
            },
            InternalNodeEvent::Children => NodeEvent::Children {
                children: NodeChildrenProcessor {
                    state: self.state,
                    in_progress: self.in_progress,
                    child_in_progress: false,
                },
            },
            InternalNodeEvent::End => {
                *self.in_progress = false;
                NodeEvent::End
            }
        })
    }
}

#[derive(Debug)]
pub struct NodeChildrenProcessor<'i, 'p> {
    state: &'p mut &'i str,

    /// Bool owned by the parent's list processor. Must be set to true only when
    /// this list has been fully consumed. Additionally used to ensure that
    /// `next_node` exhibits fused behavior.
    in_progress: &'p mut bool,

    /// Bool that ensures that node processors fully consume their nodes, so
    /// that parse state remains consistent. Set to false when a node processor
    /// is returns, and only resets to true when that processor is finished.
    child_in_progress: bool,
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
        // If the *parent* is at a node boundary, it means that *this*
        // set of children was previously completed.
        if *self.in_progress {
            return Ok(None);
        }

        // We *must* be at a node boundary internally; if we're not, it means one of our
        // children wasn't fully consumed.
        if !self.child_in_progress {
            panic!(
                "Called next_node before the previous node was fully parsed. This is a kaydle bug."
            )
        }

        match run_parser_on(self.state, parse_node_start(char('}').value(())))? {
            // None here means that we successfully parsed the end-of-children. Inform the parent.
            None => {
                *self.in_progress = false;
                Ok(None)
            }
            Some(name) => {
                self.child_in_progress = true;

                Ok(Some(NodeItem {
                    name,
                    node: NodeProcessor {
                        state: self.state,
                        in_progress: &mut self.child_in_progress,
                    },
                }))
            }
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
