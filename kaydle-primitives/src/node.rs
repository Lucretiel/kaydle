/*!
A system for parsing entire nodes and node lists. Unlike the rest of
`kaydle-primitives`, this module doesn't expose ordinary nom parsers and
container types. Instead, because it's designed specifically to support the
implementation of a serde deserializer, it exposes a set of "processors", which
can be used to fetch nodes or the contents of nodes step-by-step.

This module makes extensive use of the builder traits defined in
`kaydle-primitives` (such as [`ValueBuilder`] and [`StringBuilder`]) to allow
callers to precisely control how much information they need from the node.
Often you can use `()` instead of a real KDL type if you don't care about a
value; this will be faster to parse.

This module tries to be as misuse resistant as possible, using borrowing and
move semantics to ensure that methods aren't called out of order. Where
build-time correctness is impossible, it instead uses runtime tracking and
panics to ensure consistent state.
*/

use std::char::CharTryFromError;

use nom::{
    branch::alt,
    character::complete::char,
    combinator::eof,
    error::{FromExternalError, ParseError},
    Err as NomErr, IResult, Parser,
};
use nom_supreme::{context::ContextError, tag::TagError, ParserExt};

use crate::{
    annotation::{with_annotation, AnnotationBuilder, GenericAnnotated, RecognizedAnnotation},
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
    E: ContextError<&'i str, &'static str>,
{
    with_annotation(parse_identifier)
        .map(Some)
        .context("node")
        .or(end_of_nodes.map(|()| None))
        .preceded_by(parse_linespace)
}

/// A single node. Contains the annotated name of the node as well as a
/// [`NodeContent`], which is used to extract the arguments, properties, and
/// children from the node.
#[derive(Debug)]
pub struct Node<'i, 'a, Name> {
    /// The name of this node
    pub name: Name,

    /// A processor, used for getting the contents of the node.
    pub content: NodeContent<'i, 'a>,
}

/// The outcome of a drain operation, indicating if the thing being drained
/// was already empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainOutcome {
    /// The thing was already empty
    Empty,

    /// There was unconsumed content in the thing
    NotEmpty,
}

/// A recognized node. Used in the case where the caller cares *that* a node
/// was successfully parsed, but not what the actual value of the node is.
///
/// Note that nodes are parsed lazily, so this really only recognizes the node
/// name and annotation.
pub type RecognizedNode<'i, 'a> = Node<'i, 'a, ()>;

/// Trait for types that contain a node list. Abstracts over a [`Document`],
/// which operates at the top level, and [`Children`] which are nested in `{ }`.
pub trait NodeList<'i>: Sized {
    /// Get the next node. Returns the [`Node`], if any, which includes the
    /// name of the node as well as its [content][NodeContent].
    ///
    /// Note for implementors: this method should be fused, to ensure that
    /// `drain` is always safe to call. After it returns Ok(None), it should
    /// continue to return Ok(None) forever.
    fn next_node<'s, Annotation, Name, E>(
        &'s mut self,
    ) -> Result<Option<GenericAnnotated<Annotation, Node<'i, 's, Name>>>, NomErr<E>>
    where
        Annotation: AnnotationBuilder<'i>,
        Name: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str, &'static str>;

    /// Drain all remaining content from this nodelist. The nodelist is parsed,
    /// and errors are returned, but the nodes are otherwise discarded.
    ///
    /// Returns [`DrainOutcome::NotEmpty`] if there is at least 1 node returned
    /// by [`next_node`][Self::NextNode].
    fn drain<E>(mut self) -> Result<DrainOutcome, NomErr<E>>
    where
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: FromExternalError<&'i str, BoundsError>,
        E: ContextError<&'i str, &'static str>,
    {
        match self.next_node()? {
            None => Ok(DrainOutcome::Empty),
            Some(RecognizedAnnotation {
                item: RecognizedNode { content, .. },
                ..
            }) => {
                content.drain()?;

                while let Some(RecognizedAnnotation {
                    item: RecognizedNode { content, .. },
                    ..
                }) = self.next_node()?
                {
                    content.drain()?;
                }

                Ok(DrainOutcome::NotEmpty)
            }
        }
    }
}

impl<'i, T: NodeList<'i>> NodeList<'i> for &mut T {
    fn next_node<'s, Annotation, Name, E>(
        &'s mut self,
    ) -> Result<Option<GenericAnnotated<Annotation, Node<'i, 's, Name>>>, NomErr<E>>
    where
        Annotation: AnnotationBuilder<'i>,
        Name: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str, &'static str>,
    {
        T::next_node(*self)
    }
}

/// Container for a top level kdl document. Returns the nodes in the document.
#[derive(Debug, Clone)]
pub struct Document<'i> {
    /// The currently unparsed input string, as a suffix of the original input.
    state: &'i str,

    /// Bool that ensures that node processors fully consume their nodes, so
    /// that parse state remains consistent. Set to true when a node processor
    /// is returned, and only resets to false when that processor is finished.
    child_in_progress: bool,
}

impl<'i> Document<'i> {
    /// Create a new `Document` from an input string.
    pub fn new(input: &'i str) -> Self {
        Self {
            state: input,
            child_in_progress: false,
        }
    }

    fn run_parser<T, E>(&mut self, parser: impl Parser<&'i str, T, E>) -> Result<T, NomErr<E>> {
        run_parser_on(&mut self.state, parser)
    }
}

impl<'i> NodeList<'i> for Document<'i> {
    fn next_node<'s, Annotation, Name, E>(
        &'s mut self,
    ) -> Result<Option<GenericAnnotated<Annotation, Node<'i, 's, Name>>>, NomErr<E>>
    where
        Annotation: AnnotationBuilder<'i>,
        Name: StringBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str, &'static str>,
    {
        if self.child_in_progress {
            panic!(
                "Called next_node before the previous node was fully parsed. This is a kaydle bug."
            )
        }

        self.run_parser(parse_node_start(eof.value(())))
            .map(move |opt_name| {
                opt_name.map(move |annotated_name| {
                    self.child_in_progress = true;

                    annotated_name.map_item(|name| Node {
                        name,
                        content: NodeContent {
                            state: &mut self.state,
                            in_progress: &mut self.child_in_progress,
                        },
                    })
                })
            })
    }
}

enum InternalNodeEvent<
    ArgumentAnnotation,
    Argument,
    PropertyKey,
    PropertyValueAnnotation,
    PropertyValue,
> {
    Argument(GenericAnnotated<ArgumentAnnotation, Argument>),
    Property(GenericProperty<PropertyKey, PropertyValueAnnotation, PropertyValue>),
    Children,
    End,
}

/// A piece of content from a node.
#[derive(Debug)]
pub enum NodeEvent<
    'i,
    'p,
    ArgumentAnnotation,
    Argument,
    PropertyKey,
    PropertyValueAnnotation,
    PropertyValue,
> {
    /// An argument from a node
    Argument {
        /// The value, with its annotation
        argument: GenericAnnotated<ArgumentAnnotation, Argument>,
        /// The processor containing the rest of the node
        tail: NodeContent<'i, 'p>,
    },

    /// A property (key-value pair) from a node
    Property {
        /// The property
        property: GenericProperty<PropertyKey, PropertyValueAnnotation, PropertyValue>,
        /// The processor containing the rest of the node
        tail: NodeContent<'i, 'p>,
    },

    /// A set of children from the node.
    Children {
        /// A `NodeListProcessor` used to get child nodes one-by-one
        children: Children<'i, 'p>,
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
    E: ContextError<&'i str, &'static str>,
{
    alt((
        // Parse a value or property, preceded by 1 or more whitespace
        alt((
            // Important: make sure to try to parse a property first, since
            // "abc"=10 could be conservatively parsed as just the value "abc"
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
pub struct NodeContent<'i, 'p> {
    // The state of the original document; owned by a `Document`.
    state: &'p mut &'i str,

    /// Bool owned by the parent's list processor. Must be set to false only when
    /// this node has been fully consumed.
    in_progress: &'p mut bool,
}

impl<'i, 'p> NodeContent<'i, 'p> {
    /// Get the next piece of content from a node. This can be an argument,
    /// a property, a set of children, or [`End`][NodeEvent::End] if the node
    /// is done.
    ///
    /// For correctness, this method is move oriented. If the event is an
    /// argument or property, the event includes a new [`NodeContent`] for
    /// fetching the rest of the node. Conversely, if the event is children or
    /// the end of the node, the processor is consumed, because there's nothing
    /// more that can be retrieved from this node.
    pub fn next_event<
        ArgumentAnnotation,
        Argument,
        PropertyKey,
        PropertyValueAnnotation,
        PropertyValue,
        Error,
    >(
        mut self,
    ) -> Result<
        NodeEvent<
            'i,
            'p,
            ArgumentAnnotation,
            Argument,
            PropertyKey,
            PropertyValueAnnotation,
            PropertyValue,
        >,
        NomErr<Error>,
    >
    where
        Argument: ValueBuilder<'i>,
        ArgumentAnnotation: AnnotationBuilder<'i>,
        PropertyKey: StringBuilder<'i>,
        PropertyValue: ValueBuilder<'i>,
        PropertyValueAnnotation: AnnotationBuilder<'i>,
        Error: ParseError<&'i str>,
        Error: TagError<&'i str, &'static str>,
        Error: FromExternalError<&'i str, CharTryFromError>,
        Error: FromExternalError<&'i str, BoundsError>,
        Error: ContextError<&'i str, &'static str>,
    {
        // Because we use a move-oriented interface, there's no need to check
        // in_progress. We (or the children processor we return) just need to
        // make sure it's reset to false when we're done.
        self.run_parser(parse_node_event)
            .map(move |event| match event {
                InternalNodeEvent::Argument(argument) => NodeEvent::Argument {
                    argument,
                    tail: self,
                },
                InternalNodeEvent::Property(property) => NodeEvent::Property {
                    property,
                    tail: self,
                },
                InternalNodeEvent::Children => NodeEvent::Children {
                    children: Children {
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

    /// Parse and discard everything in this node. Checks for parse errors, but
    /// otherwise discards all data. Returns [`DrainOutcome::Empty`] unless
    /// there are any remaining properties, arguments, or non-empty children
    /// in this node.
    pub fn drain<E>(mut self) -> Result<DrainOutcome, NomErr<E>>
    where
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,
        E: FromExternalError<&'i str, CharTryFromError>,
        E: FromExternalError<&'i str, BoundsError>,
        E: ContextError<&'i str, &'static str>,
    {
        self = match self.next_event()? {
            RecognizedNodeEvent::Argument { tail, .. } => tail,
            RecognizedNodeEvent::Property { tail, .. } => tail,
            RecognizedNodeEvent::Children { children } => return children.drain(),
            RecognizedNodeEvent::End => return Ok(DrainOutcome::Empty),
        };

        loop {
            self = match self.next_event()? {
                RecognizedNodeEvent::Argument { tail, .. } => tail,
                RecognizedNodeEvent::Property { tail, .. } => tail,
                RecognizedNodeEvent::Children { children } => {
                    children.drain()?;
                    return Ok(DrainOutcome::NotEmpty);
                }
                RecognizedNodeEvent::End => return Ok(DrainOutcome::NotEmpty),
            }
        }
    }

    fn run_parser<T, E>(&mut self, parser: impl Parser<&'i str, T, E>) -> Result<T, NomErr<E>> {
        run_parser_on(self.state, parser)
    }
}

/// Processor for child nodes of a particular node (contained in `{ }`).
/// Returns the child nodes.
#[derive(Debug)]
pub struct Children<'i, 'p> {
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

impl<'i> Children<'i, '_> {
    fn run_parser<T, E>(&mut self, parser: impl Parser<&'i str, T, E>) -> Result<T, NomErr<E>> {
        run_parser_on(self.state, parser)
    }
}

impl<'i> NodeList<'i> for Children<'i, '_> {
    fn next_node<'s, A, N, E>(
        &'s mut self,
    ) -> Result<Option<GenericAnnotated<A, Node<'i, 's, N>>>, nom::Err<E>>
    where
        N: StringBuilder<'i>,
        A: AnnotationBuilder<'i>,
        E: ParseError<&'i str>,
        E: TagError<&'i str, &'static str>,

        E: FromExternalError<&'i str, CharTryFromError>,
        E: ContextError<&'i str, &'static str>,
    {
        // If the *parent* is at a node boundary, it means that *this*
        // set of children was previously completed.
        if !*self.in_progress {
            return Ok(None);
        }

        // We *must* be at a node boundary internally; if we're not, it means one of our
        // children wasn't fully consumed.
        if self.child_in_progress {
            panic!(
                "Called next_node before the previous node was fully parsed. This is a kaydle bug."
            )
        }

        self.run_parser(parse_node_start(char('}').value(())))
            .map(|opt_name| match opt_name {
                // None here means that we successfully parsed the end-of-children. Inform the parent.
                None => {
                    *self.in_progress = false;
                    None
                }
                Some(annotated_name) => {
                    self.child_in_progress = true;

                    Some(annotated_name.map_item(|name| Node {
                        name,
                        content: NodeContent {
                            state: self.state,
                            in_progress: &mut self.child_in_progress,
                        },
                    }))
                }
            })
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

    let processor = Document::new(content);

    let res: Result<DrainOutcome, nom::Err<()>> = processor.drain();
    assert_eq!(res.expect("parse error"), DrainOutcome::NotEmpty);
}
