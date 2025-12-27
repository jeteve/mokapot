use std::{fmt::Display, hash::Hash};

use h3o::LatLng;

use crate::{
    geotools::Meters,
    models::{queries::common::DocMatcher, types::OurStr},
};

use nom::{
    IResult, Parser,
    character::complete::{char, u64},
    number::complete::double,
    sequence::preceded,
};

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct LatLngWithinQuery {
    field: OurStr,
    latlng: LatLng,
    within: Meters,
}

impl Display for LatLngWithinQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} LATLNG_WITHIN {},{}",
            self.field,
            self.latlng.to_string(),
            self.within.0
        )
    }
}

// Use the string representation for hashing.
impl Hash for LatLngWithinQuery {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_string().hash(state);
    }
}

impl LatLngWithinQuery {
    /// Constructor
    pub(crate) fn new<T: Into<OurStr>>(field: T, latlng: LatLng, within: Meters) -> Self {
        LatLngWithinQuery {
            field: field.into(),
            latlng,
            within,
        }
    }

    /// The field
    pub(crate) fn field(&self) -> OurStr {
        self.field.clone()
    }

    pub(crate) fn latlng(&self) -> LatLng {
        self.latlng
    }

    pub(crate) fn within(&self) -> Meters {
        self.within
    }
}

pub(crate) fn parse_latlng(input: &str) -> Option<LatLng> {
    let mut parser = (double, preceded(char(','), double));

    let pres: IResult<&str, (f64, f64)> = parser.parse(input);

    if let Ok(out) = pres
        && let Ok(ll) = LatLng::new(out.1.0, out.1.1)
    {
        return Some(ll);
    }

    None
}

// Silently fails to parse a lat,lng,within
pub(crate) fn parse_latlng_within(input: &str) -> Option<(LatLng, Meters)> {
    let mut parser = (
        double,
        preceded(char(','), double),
        preceded(char(','), u64),
    );

    let pres: IResult<&str, (f64, f64, u64)> = parser.parse(input);

    if let Ok(out) = pres
        && let Ok(ll) = LatLng::new(out.1.0, out.1.1)
    {
        return Some((ll, Meters(out.1.2)));
    }

    None
}

// The cell value must be a valid double,double representing
// a lattitude,longitude pair.
fn _latlng_within(doc_value: &OurStr, q: &LatLngWithinQuery) -> bool {
    parse_latlng(doc_value).is_some_and(|ll| dbg!(ll.distance_m(q.latlng)) <= q.within.0 as f64)
}

impl DocMatcher for LatLngWithinQuery {
    fn matches(&self, d: &crate::prelude::Document) -> bool {
        // Try parsing all the d fields into LatLng
        d.values_iter(&self.field)
            .is_some_and(|mut i| i.any(|v| _latlng_within(&v, self)))
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::Document;

    use super::*;

    #[test]
    fn test_parse_ll_within() {
        assert!(parse_latlng_within("").is_none());
        assert!(parse_latlng_within("0,").is_none());
        assert!(parse_latlng_within("0,0").is_none());
        assert!(parse_latlng_within("NaN,0,0").is_none());
        assert!(parse_latlng_within("0,0,0").is_some());
        assert!(parse_latlng_within("0,0,1").is_some());
        assert!(parse_latlng_within("-0.1,0.1,1").is_some());
        assert!(parse_latlng_within("48.864716,2.349014,1000").is_some());
    }

    #[test]
    fn test_parse_ll() {
        assert!(parse_latlng("").is_none());
        assert!(parse_latlng("0,").is_none());
        assert!(parse_latlng("0,0").is_some());
        assert!(parse_latlng("NaN,0").is_none());
        assert!(parse_latlng("0,NaN").is_none());
        assert!(parse_latlng("48.864716,2.349014").is_some());
    }

    #[test]
    fn test_doc_matching() {
        let q = LatLngWithinQuery::new(
            "location",
            LatLng::new(48.864716, 2.349014).unwrap(),
            Meters(1000),
        );

        let d = Document::default();
        assert!(!q.matches(&d));

        let d = Document::default().with_value("location", "48.864716,2.349014");
        assert!(q.matches(&d));

        // Just inside the circle (Use https://www.freemaptools.com/radius-around-point.htm#)
        let d = Document::default().with_value("location", "48.865008,2.344328");
        assert!(q.matches(&d));

        // Still inside the circle
        let d = Document::default().with_value("location", "48.863935,2.347246");
        assert!(q.matches(&d));

        // Now one that is outside.
        let d = Document::default().with_value("location", "48.860258,2.333652");
        assert!(!q.matches(&d));
    }
}
