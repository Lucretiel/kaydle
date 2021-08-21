mod serde;
mod primitives;

trait NodeListVisitor {
    type Error;
    type NodeVisitor;

    fn visit_node(&mut self, name: &str) -> Result<Self::NodeVisitor, Self::Error>;
}

enum KdlString {
    Borrowed()
}

trait NodeVisitor {

}
