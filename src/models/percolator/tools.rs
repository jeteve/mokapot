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

#[derive(Clone, Debug)]
pub(crate) struct PreHeater {
    pub(crate) id: Rc<str>,
    pub(crate) expand_clause: ClauseExpander,
    pub(crate) must_filter: bool, // must_filter MUST be true when the clause expander is not exact.
}

impl PreHeater {
    pub(crate) fn new(id: Rc<str>, ce: ClauseExpander) -> Self {
        Self {
            id,
            expand_clause: ce,
            must_filter: false,
        }
    }
    pub(crate) fn with_must_filter(mut self, new_bool: bool) -> Self {
        self.must_filter = new_bool;
        self
    }
}
