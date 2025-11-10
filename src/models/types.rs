/**
 * Type aliases for commonly used types.
*/
pub(crate) type OurRc<T> = std::rc::Rc<T>;
pub(crate) type OurStr = OurRc<str>;
