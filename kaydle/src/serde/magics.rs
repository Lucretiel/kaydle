/// Kaydle magic for extracting an annotation
pub const ANNOTATION: &str = "$kaydle::annotation";

/// Kaydle magic for extracting the name of a node
pub const NODE_NAME: &str = "$kaydle::name";

/// Kaydle magic for extracting the properties of a node
pub const PROPERTIES: &str = "$kaydle::properties";

/// Kaydle magic for extracting the arguments of a node
pub const ARGUMENTS: &str = "$kaydle::arguments";

/// Kaydle magic for extracting the children of a node
pub const CHILDREN: &str = "$kaydle::children";

/// Kaydle magic for forwarding the entire node to some inner struct field
pub const TRANSPARENT: &str = "$kaydle::transparent";
