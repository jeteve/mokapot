#[cfg(feature = "send")]
pub(crate) type OurRc<T> = std::sync::Arc<T>;

#[cfg(not(feature = "send"))]
pub(crate) type OurRc<T> = std::rc::Rc<T>;

pub(crate) type OurStr = OurRc<str>;
