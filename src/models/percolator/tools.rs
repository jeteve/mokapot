use std::{fmt, rc::Rc};

use crate::models::cnf::Clause;

pub(crate) type ExpanderF = Rc<dyn Fn(Clause) -> Clause>;

#[derive(Clone)]
// Extends Clauses comming from percolated document with extra termqueries.
pub(crate) struct ClauseExpander(pub(crate) ExpanderF);
impl std::fmt::Debug for ClauseExpander {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ClauseExpander")
            .field(&"_OPAQUE FUNCTION_")
            .finish()
    }
}
impl ClauseExpander {
    pub(crate) fn new(f: ExpanderF) -> Self {
        Self(f)
    }
}
