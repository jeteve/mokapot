use itertools::Itertools;
use mokapot::prelude::*;

#[test]
fn test_percolator() {
    let mut p = Percolator::default();
    let some_q = (!"A".has_value("a")) | "B".has_value("b");
    let q: Vec<Qid> = vec![
        p.add_query("A".has_value("a")),
        p.add_query("A".has_value("a") | "B".has_value("b")),
        p.add_query("A".has_value("a") & "B".has_value("b")),
        p.add_query(!"A".has_value("a")),
        // TODO: Reinstate that. But the negative matching will have to change.
        p.add_query(some_q.clone()),
    ];

    assert_eq!(some_q.to_string(), "(AND (OR ~A=a B=b))");
    // Means A=a -> B=b
    // A document with A=a MUST have B=b to match.
    // Means we can NOT exclude the qids matching A=a in the 'negative index'
    // A document with !A=a should match, regardless of B value or not.

    // This should also match 4
    assert_eq!(
        p.percolate(&Document::default().with_value("X", "x"))
            .collect_vec(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&Document::default().with_value("A", "b"))
            .collect_vec(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&Document::default().with_value("A", "a"))
            .collect_vec(),
        vec![q[0], q[1]]
    );

    assert_eq!(
        p.percolate(
            &Document::default()
                .with_value("A", "a")
                .with_value("B", "b")
        )
        .collect_vec(),
        vec![q[0], q[1], q[2], q[4]]
    );
}
