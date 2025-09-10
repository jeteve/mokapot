use itertools::Itertools;
use mokapot::prelude::*;

#[test]
fn test_percolator() {
    let mut p = Percolator::default();
    let q: Vec<Qid> = vec![
        p.add_query("A".has_value("a")),
        p.add_query("A".has_value("a") | "B".has_value("b")),
        p.add_query("A".has_value("a") & "B".has_value("b")),
    ];

    assert_eq!(
        p.percolate(&Document::default().with_value("X", "x"))
            .collect_vec(),
        vec![]
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
        vec![q[0], q[1], q[2]]
    );
}
