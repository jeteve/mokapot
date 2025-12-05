use h3o::CellIndex;

use crate::{
    models::{queries::common::DocMatcher, types::OurRc, types::OurStr},
    prelude::Document,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct H3InsideQuery {
    field: OurStr,
    cell: CellIndex,
}

impl H3InsideQuery {
    /// Constructor
    pub(crate) fn new<T: Into<OurStr>>(field: T, cell: CellIndex) -> Self {
        H3InsideQuery {
            field: field.into(),
            cell,
        }
    }

    /// The field
    pub(crate) fn field(&self) -> OurStr {
        self.field.clone()
    }

    /// The H3 CellIndex
    pub(crate) fn cell(&self) -> CellIndex {
        self.cell
    }
}

// Free function to test a string from a potential string CellIndex to
// a parent.
fn _has_parent(cell_str: &OurRc<str>, parent: CellIndex) -> bool {
    cell_str.as_ref().parse::<CellIndex>().is_ok_and(|cell| {
        cell.parent(parent.resolution())
            .is_some_and(|ancestor| ancestor.eq(&parent))
    })
}

impl DocMatcher for H3InsideQuery {
    /// Does this match the document?
    fn matches(&self, d: &Document) -> bool {
        d.values_iter(&self.field)
            .is_some_and(|mut i| i.any(|v| _has_parent(&v, self.cell())))
    }
}

#[cfg(test)]
mod test_h3_inside {
    use h3o::CellIndex;

    use crate::prelude::Document;

    use super::*;

    #[test]
    fn test_doc_matching() {
        let q = H3InsideQuery::new("location", "87194d106ffffff".parse::<CellIndex>().unwrap());
        assert!(q.field().eq(&"location".into()));
        assert!(!q.matches(&Document::default()));
        assert!(!q.matches(&[("some", "thing")].into()));
        assert!(!q.matches(&[("field", "pr")].into()));

        assert!(q.matches(&[("location", "87194d106ffffff")].into()));
        assert!(q.matches(&[("location", "88194d1069fffff")].into()));
        assert!(q.matches(&[("location", "89194d10693ffff")].into()));
    }

    #[test]
    fn test_has_parent() {
        // Find some examples there:
        // https://observablehq.com/@nrabinowitz/h3-index-inspector?collection=@nrabinowitz/h3
        assert!(_has_parent(
            &"87194d106ffffff".into(),
            "87194d106ffffff".parse::<CellIndex>().unwrap()
        ));
        assert!(!_has_parent(
            &"sausage".into(),
            "87194d106ffffff".parse::<CellIndex>().unwrap()
        ));

        assert!(_has_parent(
            &"87194d106ffffff".into(),
            "86194d107ffffff".parse::<CellIndex>().unwrap()
        ));

        // But not the other way around:
        assert!(!_has_parent(
            &"86194d107ffffff".into(),
            "87194d106ffffff".parse::<CellIndex>().unwrap()
        ));
    }
}
