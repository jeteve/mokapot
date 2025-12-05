use std::num::NonZeroUsize;

use mokaccino::prelude::*;

#[test]
fn test_percolator() {
    test_nclause_percolator(NonZeroUsize::new(1).unwrap());

    test_nclause_percolator(NonZeroUsize::new(2).unwrap());

    test_nclause_percolator(NonZeroUsize::new(3).unwrap());

    test_nclause_percolator(NonZeroUsize::new(5).unwrap());
}

#[test]
#[cfg(feature = "serde")]
fn test_serialisation() {
    let mut p = Percolator::default();
    let qids: Vec<Qid> = vec![
        p.add_query("A".has_value("a")),                      //0
        p.add_query("A".has_value("a") | "B".has_value("b")), //1
        p.add_query("A".has_value("a") & "B".has_value("b")), //2
        p.add_query(!"A".has_value("a")),                     //3
        p.add_query("A".i64_lt(10000)),
    ];

    let json = serde_json::to_string(&p).unwrap();
    println!("{}", json);
    let p2: Percolator = serde_json::from_str(&json).unwrap();
    for qid in qids {
        // No crash. Query is still there!
        let _ = p2.get_query(qid);
    }
}

fn test_nclause_percolator(n: NonZeroUsize) {
    let mut p = Percolator::builder().n_clause_matchers(n).build();

    let q: Vec<Qid> = vec![
        p.add_query("A:a".parse().unwrap()),                     //0
        p.add_query("A:a OR B:b".parse().unwrap()),              //1
        p.add_query("A".has_value("a") & "B".has_value("b")),    //2
        p.add_query(!"A".has_value("a")),                        //3
        p.add_query((!"A".has_value("a")) | "B".has_value("b")), //4
        p.add_query(!"A".has_value("a") & "B".has_value("b")),   //5
        p.add_query(!"A".has_value("a") & "A".has_value("a")),   //6 - should NEVER match anything.
        p.add_query("C".has_prefix("multi")),                    //7
        p.add_query("C".has_prefix("multi") & !"C".has_value("multimeter")), //8
        p.add_query(
            "A".has_value("aa") & "B".has_value("bb") & "C".has_value("cc") & "D".has_prefix("bla"),
        ), //9
        p.add_query("P".has_prefix("")),                         // 10 P can mean Path
        p.add_query("P".i64_gt(1000)),                           // 11 P can mean Price too!
        p.add_query("W".i64_lt(10)),                             // 12 W for weight
        p.add_query("W".i64_le(10)),                             // 13
        p.add_query("W".i64_ge(2000)),                           // 14
        p.add_query("W".i64_eq(12345)),                          // 15
        p.add_query("position".h3in("871f09b20ffffff".parse().unwrap())), // 16 something in gdansk old town
    ];

    assert_eq!(
        // Invalid position.. Cannot be matched against a h3 CellIndex
        p.percolate(&[("position", "bla")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        // Valid position equal to the cell index
        p.percolate(&[("position", "871f09b20ffffff")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4], q[16]]
    );

    assert_eq!(
        // Valid position inside the query
        // Use https://observablehq.com/@nrabinowitz/h3-index-inspector?collection=@nrabinowitz/h3
        p.percolate(&[("position", "881f09b203fffff")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4], q[16]]
    );

    assert_eq!(
        // Valid position outside the query
        // Use https://observablehq.com/@nrabinowitz/h3-index-inspector?collection=@nrabinowitz/h3
        // Actually something in a neighbour
        p.percolate(&[("position", "881f09b211fffff")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        // Valid position LARGER than the query
        // Use https://observablehq.com/@nrabinowitz/h3-index-inspector?collection=@nrabinowitz/h3
        // which means that its not possible to turn it into the query resolution.
        p.percolate(&[("position", "861f09b27ffffff")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&[("P", ""), ("P", "1001")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4], q[10], q[11]]
    );

    assert_eq!(
        p.percolate(&[("W", "Not an int")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&[("W", "10")].into()).collect::<Vec<_>>(), // 10 does not yield query 12
        vec![q[3], q[4], q[13]]
    );

    assert_eq!(
        p.percolate(&[("W", "0009")].into()).collect::<Vec<_>>(), // 9 does!
        vec![q[3], q[4], q[12], q[13]]
    );
    assert_eq!(
        p.percolate(&[("W", "-123")].into()).collect::<Vec<_>>(), // As well as a negative number
        vec![q[3], q[4], q[12], q[13]]
    );

    assert_eq!(
        p.percolate(&[("W", "2000")].into()).collect::<Vec<_>>(), // As well as a negative number
        vec![q[3], q[4], q[14]]
    );
    assert_eq!(
        p.percolate(&[("W", "12345")].into()).collect::<Vec<_>>(), // As well as a negative number
        vec![q[3], q[4], q[14], q[15]]
    );

    assert_eq!(
        p.percolate(&[("P", "")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4], q[10]]
    );
    assert_eq!(
        p.percolate(&[("P", "some value")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4], q[10]]
    );

    assert_eq!(
        p.percolate(&[("A", "aa"), ("B", "bb"), ("C", "cc"), ("D", "blabla")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4], q[9]]
    );

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
