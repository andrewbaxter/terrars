use crate::StackShared;

pub trait Ref {
    fn new(shared: StackShared, base: String) -> Self;
}
