# Kaydle specification

This document describes how KDL documents are mapped to the serde data model by kaydle. Kaydle, being a node-oriented (similar to XML), doesn't directly map very well to serde's collection-oriented model (similar to JSON). The serde deserializer is therefore fairly complex. This document attempts to formally describe the mapping, as an aid to Rust type designers and as a target against which to file bugs or propose improvements.

## KDL refresher

This section is a refresher on how KDL works; make sure to check out the [official specification](https://github.com/kdl-org/kdl/blob/main/SPEC.md) for more details. This section is mostly a reminder for the data types and semantics of KDL; it's not concerned with specifics about syntax / parsing, comments, etc.

1. A KDL document is a list of Nodes.
1. A KDL Node consists of a name, a list of 0 or more Arguments and 0 or more Properties, followed by an optional set of Children. Arguments and properties are separated by whitespace and may be interspersed, but semantically are separate.
1. Children are a list of Nodes associated with some parent Node.
1. A Node name is an Identifier.
1. A Node argument is any Value associated with a Node. Arguments are semantically ordered.
1. A Property is a key-value pair associated with a Node, where the key is an Identifier and the value is any Value. Properties are semantically unordered and later properties override earlier ones. (`a=1 b=2` is equivalent to `b=3 b=2 a=1`)
1. A Value is any one of `null`, `true`, `false`, a number, or a String.
1. KDL numbers may be binary, octal, decimal, or hex; decimal numbers may have a fractional and/or exponent part. KDL numbers have unlimited precision and unlimited width; all forms of the same number are considered semantically identical.
1. KDL has 3 kinds of string: Quoted, Raw, and Identifier. All of these strings are considered semantically identical, but the language syntax specifies with types of strings may be used in which syntax positions.
   - For succinctness, `kaydle` defines the following hierarchy of strings:
     - KDL strings are defined as Escaped Strings, Raw Strings, and Bare Identifiers.
     - A String is a Raw String or Escaped String.
     - An Identifier is a String or Bare Identifier.
1. Any Node or Value may be prefixed with an Annotation, which is an Identifier.

## Specification

### Kaydle Entities

At a high level, kaydle interprets KDL documents in terms of three types of structure:

#### Nodelist

A Nodelist is an ordered collection of KDL nodes; it is either the top-level KDL document, or a set of child nodes associated with a particular node.

#### Named Node

A Named Node is a KDL Node that is being processed _with_ its name. How it is interpreted depends on the context, but as an example, the node name might be used as an enum discriminant.

#### Anonymous Node

An Anonymous Node is a KDL Node that is being processed without its name. How it is interpreted depends on the context, but as an example, a Nodelist that is being interpreted as a map will use a node name as a map key, and will use the _rest_ of the node (that is, the Anonymous Node) as the map value.

### Magics

kaydle magics are specialized field names resembling `$kaydle::magic` that, when present in a struct type, allow kaydle to capture more complex KDL data that it would otherwise have to reject as "too ambiguous". For instance, kaydle normally must reject nodes that contain both properties and children (since there's no reasonable mapping of these nodes into the serde data), but it can successfully deserialize such a type into this `struct`:

```rust
#[derive(Deserialize)]
struct PropsAndChildren {
    #[serde(rename="$kaydle::properties")]
    properties: Pair,

    #[serde(rename="$kaydle::children")]
    children: HashMap<String, i32>,,
}

#[derive(Deserialize)]
struct Pair {
    #[serde(default)]
    a: Option<i32>,

    #[serde(default)]
    b: String
}
```

### Behavior

- When kaydle encounters a Nodelist (which is either a top-level Document or set of Children):
  - If a serde map or struct is requested, the Nodes are treated as key-value pairs, where the node's name is the key, and the value is the Anonymous Node.
  - If a serde sequence / tuple type is requested, the Nodes are treated as an ordered sequence, where each element is a Named Node
  - Other types are errors, including `any`.
- When kaydle encounters a Named Node:
  - If the requested type is an enum, the node name is used as the enum variant selector, and the variant's content is deserialized as though this was an Anonymous node.
    - For instance, a tuple_variant will be deserialized as a tuple struct via the anonymous node.
  - If the requested type is named and not an enum (such as a struct, newtype struct, or unit struct), the node name must match the name of the type; mismatches are an error. The type is then deserialized as though this was an
    anonymous node
    - Exception: if the type is a struct type and includes a `$kaydle::name` magic, the type's name is ignored, and the node's name is deserialized into that field. If this type additionally has a `$kaydle::transparent` magic, that field is used as the target for the anonymous node deserialize. This serves the same purpose as `#[serde(transparent)]` in cases where you also want `$kaydle::name`. Additional fields are an error in this case.
  - If the requested type is _not_ named, the node name _must_ be `-`; mismatches are an error. The type is then deserialized as though this was an anonymous node.
- When kaydle encounters an anonymous node:
  - If the requested type is a struct, first check for magics:
    - `$kaydle::properties`: the collected set of properties of the node, as a serde map.
    - `$kaydle::arguments`: the collected set of arguments, as a serde sequence
    - `$kaydle::children`: the collected set of children, as either a map or a sequence.
    - `$kaydle::annotation`: the annotation associated with this node, as a string.
    - kaydle will ignore magically collected data when deciding a behavior. For instance, it will normally fail to deserialize if a node has both children and properties, but it will _succeed_ if those properties are collected into a magic and treat the node as though it only had children.
  - If the requested type is a mapping type, the node must have either properties or children (or neither), and must not have arguments.
    - If it has properties, they are deserialized as a map
    - If it has children, they are deserialized as a map, using the Node names as keys and the Anonymous Nodes as values.
  - If the requested type is a sequence or tuple type, the node must have either arguments or children (or neither), and must not have children.
    - If it has values, they are deserialized as a sequence.
    - If it has children, they are deserialized as a sequence, using the Named Nodes as values
  - If the requested type is an enum, the first argument to the node is used as the enum variant, and the remainder of the node is used to deserialize the content of the enum (as though it was an anonymous node without that first argument)
  - If the requested type is a unit, the node must have no arguments, properties, or children (not even `null`).
  - If the requested type is an option, and it has no children or properties, and it has no arguments or a single `null` argument, it's deserialized as `none`; otherwise, it's deserialized normally.
  - If the requested type is a primitive, the node must have exactly 1 argument, no properties, and no children. The argument is deserialized directly.
  - Other types are errors, including `any`.
- When kaydle encounters a Value:
  - If the requested type is an option, `null` is deserialized; otherwise, the value is forwarded.
  - If the requested type is a struct, and it has a `$kaydle::annotation` field and exactly one other field, the annotation for the value is extracted and the value is forwarded. Other kinds of structs use the catch-all Value rule.
  - For all other types (including `any`), the type is deserialized based on the KDL type, without regard for the type hint:
    - Strings are strings.
    - `true` and `false` are bool.
    - `null` is a unit.
    - Numbers are forwarded. See Discussion for details on this.

### Discussion

- Except when deserializing a KDL Value or Identifier, `any` will be rejected, as kaydle depends on the type hint information to guide its behavior. In the future this restriction may be lifted for Nodes, but it's unlikely to ever be lifted for Nodelists.
- That being said, `ignored_any` is always accepted, and will discard whatever KDL "thing" is being deserialized, ignoring any of the normal kaydle restrictions.
- kaydle, unlike most serde deserializers, is sensitive to type names. It requires node names to match type names when deserializing a Named node, and it doesn't transparently forward to newtype structs (use `#[serde(transparent)]` for forwarding behavior). This is mostly intended to provide pairity with serialization, which will use type names as Node names when serializing a sequence of non-enums.
- A consequence of the rules for primitives and nodes is that "empty node" and `null` are both treated as `None`. Hypothetically, we could use these two cases to distinguish `None` (empty node) from `Some(None)` (`null`) (and in fact this would simplify the node handling rules), but in practice we assume that double options are rare in practice, and that users would be surprised to see `null` deserialized as `Some(...)`.
- KDL (unlike serde, but like most other human-readable data formats) doesn't distinguish between different kinds of number, or even between integers and floats. kaydle, therefore, uses a set of parsing rules to parse a number as either an `f64`, `i64`, or `u64`, which is then deserialized. The specific rules for this process are covered by kaydle's semver versioning but are documented separately and left deliberately unspecified in this spec.
- While kaydle magics _can_ be catch-all types (like `Vec<KdlValue>` for `$kaydle::arguments` and `HashMap<String, KdlValue>` for `$kaydle::properties`), they don't have to be. kaydle will deserialize them using with ordinary `map` and `seq` deserialization.

## KDL spec divergences

This section lists the divergences from the KDL specification that are intentional for the time being. Other divergences are considered bugs. Where possible, we include workaround or other mitigations in cases where you need spec-compliant behavior.

In general, these divergences are cases where faithfully fulfilling the spec would make a lot more work for kaydle, and we instead prefer to give the `Deserialize` type "what we have" rather than going out of our way to normalize it.

### Properties

KDL specifies that properties are unordered, and later properties override earlier properties with identical keys. For example, these Nodes are all semantically identical:

```kdl
node a=1 b=2
node b=2 a=1
node a=1 b=2 a=1
```

kaydle does not make any attempt to normalize property ordering or presence. It will pass _all_ properties to the underlying `Deserialize` in the order they're parsed, which means that a `Deserialize` implementation that is sensitive to map key order or duplicate key might behave differently depending on how the properties are written.

That being said, the "typical" `Deserialize` implementations for map types are, in practice, insensitive to key ordering. In particular, the implementations for the built-in map data structures like `HashMap` and `BTreeMap` are order-insensitive, as are the `#[derive(Deserialize)]` implementations created for `struct` types.

However, "Typical" `Deserialize` implementations do behave differently in the presence of duplicate keys. For data structures, they follow the "last key wins" behavior specified by KDL, but for `struct` types, duplicate keys are an error.

### Empty Children

KDL specifies that an empty set of Children is semantically identical to an _absent_ set of children. For example, these Nodes are semantically identical:

```kdl
node
node {}
```

In certain cases, kaydle will select a behavior based on the presence or absence of a set of Children for a node; for instance, without magics, kaydle can (with rare exceptions) only handle nodes that have properties _or_ arguments _or_ children (and will fail with an error if combinations are encountered). Currently, this is based on the presence or absence of the set of children; that is, "has no set of children" is distinct from "has a set of children that is empty":

```kdl
// kaydle is fine with this
node a=1 b=2

// without magics, kaydle will reject this
node a=1 b=2 {}
```

kaydle magics can be used to force a certain behavior, and the `$kaydle::children` property can treat an empty Children set as identical to an absent Children set.

### Property-Children and Argument-Children Equivalence

Because serde only has the concept of a map type to handle key-value pairs (and equivalent elaborations like struct and struct variant), kaydle has to interpret both properties and children as maps, where relevant. For instance, given these Rust types:

```rust
#[derive(Deserialize)]
#[serde(rename="data")]
struct Data {
    a: i32,
    b: bool,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct DataList {
    data: Vec<Data>
}
```

kaydle will successfully deserialize this document into a `DataList`, likely in contradiction with the intent of its author:

```kdl
data a=1 b=true
data {
    a 2
    b false
}
```

Similarly, kaydle has to interpret both arguments and children as sequences, where relevant. For instance, given these Rust types:

```rust
#[derive(Deserialize)]
#[serde(rename="data")]
struct Data(Vec<String>);

#[derive(Deserialize)]
#[serde(transparent)]
struct DataList {
    data: Vec<Data>
}
```

kaydle will successfully deserialize this document into a `DataList`, likely in contradiction with the intent of its author:

```kdl
data a b c
data {
  - "a"
  - "b"
  - "c"
}
```

This problem can be avoided with kaydle magics, which in general are the intended way to resolve potential ambiguities in cases where serde's data model is a subset of KDL's, such as nodes that may contain both properties and children.

### Unicode

KDL specifies many entities in terms of Unicode Code Points (eg, KDL identifiers are made up of "any code point except for ..."). Rust strings and `char` are, in contrast, made up of Unicode Scalar Values, which are a very light subset of Code Points. In practice we don't expect this will ever cause issues.

## Other KDL notes

### Annotations

While kaydle _does_ expose annotations to the user through the `$kaydle::annotation` magic, it doesn't otherwise use them to guide its behavior. In theory it could use them to distinguish certain ambiguous cases that are currently simply decided in an opinionated way (for instance, in the treatment of newtype structs with Named Nodes), but this seems like it would go against the grain of the typical use of annotations, which is to be a type description (eg, for timestamps).
