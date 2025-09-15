use mokapot::prelude::*;

#[test]
fn test_percolator() {
    let mut p = Percolator::default();

    let q: Vec<Qid> = vec![
        p.add_query("A".has_value("a")),                         //0
        p.add_query("A".has_value("a") | "B".has_value("b")),    //1
        p.add_query("A".has_value("a") & "B".has_value("b")),    //2
        p.add_query(!"A".has_value("a")),                        //3
        p.add_query((!"A".has_value("a")) | "B".has_value("b")), //4
        p.add_query(!"A".has_value("a") & "B".has_value("b")),   //5
        p.add_query(!"A".has_value("a") & "A".has_value("a")),   //6 - should NEVER match anything.
        p.add_query("C".has_prefix("multi")),                    //7
        p.add_query("C".has_prefix("multi") & !"C".has_value("multimeter")), //8
    ];

    assert_eq!(
        p.percolate(&[("C", "mult")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );
    assert_eq!(
        p.percolate(&[("C", "multimeter")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4], q[7]]
    );

    assert_eq!(
        p.percolate(&[("C", "multi")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4], q[7], q[8]]
    );

    assert_eq!(
        p.percolate(&[("X", "x")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&[("B", "b")].into()).collect::<Vec<_>>(),
        vec![q[1], q[3], q[4], q[5]]
    );

    assert_eq!(
        p.percolate(&[("A", "b")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&[("A", "a")].into()).collect::<Vec<_>>(),
        vec![q[0], q[1]]
    );

    assert_eq!(
        p.percolate(&[("A", "a"), ("B", "b")].into())
            .collect::<Vec<_>>(),
        vec![q[0], q[1], q[2], q[4]]
    );
}
