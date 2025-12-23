use std::fmt::{self, Display};

use h3o::CellIndex;

use crate::{
    models::{queries::common::DocMatcher, types::OurStr},
    prelude::Document,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct H3InsideQuery {
    field: OurStr,
    cell: CellIndex,
}

impl Display for H3InsideQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=H3IN={}", self.field(), self.cell())
    }
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

/// Free function to test a string from a potential string CellIndex to
/// a parent.
fn _has_parent(cell_str: &OurStr, parent: CellIndex) -> bool {
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
    fn test_getters_and_display() {
        let cell = "87194d106ffffff".parse::<CellIndex>().unwrap();
        let q = H3InsideQuery::new("location", cell);
        assert_eq!(q.field(), "location".into());
        assert_eq!(q.cell(), cell);
        assert_eq!(format!("{}", q), format!("location=H3IN={}", cell));
    }

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

        // Test parent that is not a direct parent but an ancestor
        assert!(_has_parent(
            &"88194d1069fffff".into(),                       // Res 8
            "86194d107ffffff".parse::<CellIndex>().unwrap()  // Res 6
        ));

        // Test with different valid cell that is NOT a child
        assert!(!_has_parent(
            &"87194d106ffffff".into(),
            "87195d106ffffff".parse::<CellIndex>().unwrap() // Different cell
        ));
    }
}
