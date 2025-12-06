// Parsing CNF queries
use chumsky::prelude::*;
use h3o::CellIndex;

use crate::{models::cnf, prelude::CNFQueryable};

impl std::str::FromStr for cnf::Query {
    type Err = String; // A newline delimited string, with all parsing errors.

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = query_parser();
        p.parse(s)
            .into_result()
            .map_err(|e| {
                e.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .map(|astq| astq.to_cnf())
    }
}

#[derive(Debug, PartialEq, Clone)]
enum QueryAST {
    Neg(Box<QueryAST>),
    Atom(String, OperatorAST, FieldValueAST),
    And(Box<QueryAST>, Box<QueryAST>),
    Or(Box<QueryAST>, Box<QueryAST>),
}

fn atom_to_cnf(field: &str, operator: &OperatorAST, field_value: &FieldValueAST) -> cnf::Query {
    match (&operator, &field_value) {
        // A prefix ALWAYS give a prefix, regardless of operator used.
        // It is a bit dirty, but will fix in the future.
        (OperatorAST::H3Inside, FieldValueAST::Term(t)) => 
         t.parse::<CellIndex>().map_or_else(|_err| field.has_value(t.clone()), |ci| field.h3in(ci))
,
        // Cannot do H3 on integers..
        (OperatorAST::H3Inside, FieldValueAST::Integer(i)) => field.has_value(i.to_string()),
        (_, FieldValueAST::Prefix(p)) => field.has_prefix(p.clone()),
        (_, FieldValueAST::Term(t)) => field.has_value(t.clone()),
        // Fallback to term style query in case there is ':123'
        (OperatorAST::Colon, FieldValueAST::Integer(i)) => field.has_value(i.to_string()),
        (OperatorAST::Lt, FieldValueAST::Integer(i)) => field.i64_lt(*i),
        (OperatorAST::Le, FieldValueAST::Integer(i)) => field.i64_le(*i),
        (OperatorAST::Eq, FieldValueAST::Integer(i)) => field.i64_eq(*i),
        (OperatorAST::Ge, FieldValueAST::Integer(i)) => field.i64_ge(*i),
        (OperatorAST::Gt, FieldValueAST::Integer(i)) => field.i64_gt(*i),
    }
}

impl QueryAST {
    fn to_cnf(&self) -> cnf::Query {
        match &self {
            QueryAST::Neg(query) => !query.to_cnf(),
            QueryAST::Atom(field, operator, field_value) => atom_to_cnf(field, operator, field_value),
            QueryAST::And(query, query1) => query.to_cnf() & query1.to_cnf(),
            QueryAST::Or(query, query1) => query.to_cnf() | query1.to_cnf(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum OperatorAST {
    Colon,
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
    H3Inside,
}

#[derive(Debug, PartialEq, Clone)]
enum FieldValueAST {
    Term(String),
    Prefix(String),
    Integer(i64),
}

type MyParseError<'src> = extra::Err<Rich<'src, char>>;

fn query_parser<'src>() -> impl Parser<'src, &'src str, QueryAST, MyParseError<'src>> {
    recursive(|expr| {
        let recursive_atom = atom_parser()
            .or(expr.delimited_by(just('('), just(')')))
            .padded();

        let unary = text::ascii::keyword("NOT")
            .padded()
            .repeated()
            .foldr(recursive_atom, |_op, rhs| QueryAST::Neg(Box::new(rhs)))
            .boxed();

        let product = unary.clone().foldl(
            text::ascii::keyword("AND")
                .to(QueryAST::And as fn(_, _) -> _)
                .then(unary)
                .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        );

        // sum has less precedence than product. So flat queries
        // is a sum of products.
        product.clone().foldl(
            text::ascii::keyword("OR")
                .to(QueryAST::Or as fn(_, _) -> _)
                .then(product)
                .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        )
    })
    .padded()
}

fn atom_parser<'src>() -> impl Parser<'src, &'src str, QueryAST, MyParseError<'src>> {
    identifier_parser()
        .then(operator_parser())
        .then(field_value_parser())
        .map(|((s, o), v)| QueryAST::Atom(s, o, v))
        .padded()
}

fn operator_parser<'src>() -> impl Parser<'src, &'src str, OperatorAST, MyParseError<'src>> {
    choice((
        just(':').to(OperatorAST::Colon),
        just("<=").to(OperatorAST::Le),
        just(">=").to(OperatorAST::Ge),
        just('<').to(OperatorAST::Lt),
        just('>').to(OperatorAST::Gt),
        just('=').to(OperatorAST::Eq),
        just("H3IN").to(OperatorAST::H3Inside),
    ))
    .padded()
}

static NON_IDENTIFIERS: [char; 11] = [' ', '\t', '\n', '"', '(', ')', ':', '*', '<', '>', '='];

fn identifier_parser<'src>() -> impl Parser<'src, &'src str, String, MyParseError<'src>> {
    none_of(NON_IDENTIFIERS)
        .filter(|c: &char| !c.is_whitespace())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .padded()
}

fn field_value_parser<'src>() -> impl Parser<'src, &'src str, FieldValueAST, MyParseError<'src>> {
    let term_char = just('\\')
        .ignore_then(any()) // After backslash, accept any character
        .or(none_of('"')); // Or any character that's not a quote, as this is meant to use into a phrase parser.

    let phrase = just('"')
        .ignore_then(term_char.repeated().collect::<String>())
        .then_ignore(just('"').labelled("closing double quote"))
        .labelled("Quote enclosed phrase")
        .then(just('*').or_not())
        .map(|(t, wc)| {
            if wc.is_some() {
                FieldValueAST::Prefix(t)
            } else {
                FieldValueAST::Term(t)
            }
        });

    let naked_string = none_of(NON_IDENTIFIERS)
        .filter(|c: &char| !c.is_whitespace())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then(just('*').or_not())
        .map(|(t, wc)| {
            if wc.is_some() {
                FieldValueAST::Prefix(t) // With a wild char, this is ALWAYS a word
            } else {
                // Attempt to parse as i64. If fail, fallback to just string.
                t.parse::<i64>()
                    .map(FieldValueAST::Integer)
                    .unwrap_or(FieldValueAST::Term(t))
            }
        });

    choice((phrase, naked_string)).padded()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_parser() {
        let p = query_parser();

        assert_eq!(
            p.parse("location H3IN blablabla").output(),
            Some(&QueryAST::Atom(
                "location".to_string(),
                OperatorAST::H3Inside,
                FieldValueAST::Term("blablabla".into())
            ))
        );

        assert_eq!(
            p.parse("location H3IN 1234").output(),
            Some(&QueryAST::Atom(
                "location".to_string(),
                OperatorAST::H3Inside,
                FieldValueAST::Integer(1234)
            ))
        );

        assert_eq!(
            p.parse("name:abc").output(),
            Some(&QueryAST::Atom(
                "name".to_string(),
                OperatorAST::Colon,
                FieldValueAST::Term("abc".into())
            ))
        );

        assert_eq!(
            p.parse("location H3IN abc")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR location=abc))"
        );

        assert_eq!(
            p.parse("location H3IN 1234")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR location=1234))"
        );

        assert_eq!(
            p.parse("location H3IN invalidh3")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR location=invalidh3))"
        );

        // And now a valid h3
        assert_eq!(
            p.parse("location H3IN 861f09b27ffffff")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR location=H3IN=861f09b27ffffff))"
        );

        assert_eq!(
            p.parse("name:abc").output().unwrap().to_cnf().to_string(),
            "(AND (OR name=abc))"
        );

        assert_eq!(
            p.parse("name:abc AND price<=123").output(),
            Some(&QueryAST::And(
                Box::new(QueryAST::Atom(
                    "name".to_string(),
                    OperatorAST::Colon,
                    FieldValueAST::Term("abc".into())
                )),
                Box::new(QueryAST::Atom(
                    "price".to_string(),
                    OperatorAST::Le,
                    FieldValueAST::Integer(123)
                ))
            ))
        );

        assert_eq!(
            p.parse("name:abc AND price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR name=abc) (OR price<=123))"
        );

        assert_eq!(
            p.parse("name:abc AND NOT price<=123 OR colour:blue*")
                .output(),
            Some(&QueryAST::Or(
                Box::new(QueryAST::And(
                    Box::new(QueryAST::Atom(
                        "name".to_string(),
                        OperatorAST::Colon,
                        FieldValueAST::Term("abc".into())
                    )),
                    Box::new(QueryAST::Neg(Box::new(QueryAST::Atom(
                        "price".to_string(),
                        OperatorAST::Le,
                        FieldValueAST::Integer(123)
                    ))))
                )),
                Box::new(QueryAST::Atom(
                    "colour".to_string(),
                    OperatorAST::Colon,
                    FieldValueAST::Prefix("blue".into())
                ))
            ))
        );

        assert_eq!(
            p.parse("name:abc AND NOT price<=123 OR colour:blue*")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR colour=blue* name=abc) (OR colour=blue* ~price<=123))"
        );

        // AND has precedence
        assert_eq!(
            p.parse("colour:blue* OR name:abc AND NOT price<=123")
                .output(),
            Some(&QueryAST::Or(
                Box::new(QueryAST::Atom(
                    "colour".to_string(),
                    OperatorAST::Colon,
                    FieldValueAST::Prefix("blue".into())
                )),
                Box::new(QueryAST::And(
                    Box::new(QueryAST::Atom(
                        "name".to_string(),
                        OperatorAST::Colon,
                        FieldValueAST::Term("abc".into())
                    )),
                    Box::new(QueryAST::Neg(Box::new(QueryAST::Atom(
                        "price".to_string(),
                        OperatorAST::Le,
                        FieldValueAST::Integer(123)
                    ))))
                ))
            ))
        );

        assert_eq!(
            p.parse("colour:blue* OR name:abc AND NOT price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR colour=blue* name=abc) (OR colour=blue* ~price<=123))"
        );

        // Beat precedence with parenthesis
        assert_eq!(
            p.parse("(colour:blue* OR name:abc) AND NOT price<=123")
                .output(),
            Some(&QueryAST::And(
                Box::new(QueryAST::Or(
                    Box::new(QueryAST::Atom(
                        "colour".to_string(),
                        OperatorAST::Colon,
                        FieldValueAST::Prefix("blue".into())
                    )),
                    Box::new(QueryAST::Atom(
                        "name".to_string(),
                        OperatorAST::Colon,
                        FieldValueAST::Term("abc".into())
                    ))
                )),
                Box::new(QueryAST::Neg(Box::new(QueryAST::Atom(
                    "price".to_string(),
                    OperatorAST::Le,
                    FieldValueAST::Integer(123)
                ))))
            ))
        );

        assert_eq!(
            p.parse("(colour:blue* OR name:abc) AND NOT price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR colour=blue* name=abc) (OR ~price<=123))"
        );

        // Try a distributed NOT
        assert_eq!(
            p.parse("NOT (colour:blue* OR name:abc) AND price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR ~colour=blue*) (OR ~name=abc) (OR price<=123))"
        );

        assert_eq!(
            p.parse("NOT (colour:blue* AND name:abc) OR price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR ~colour=blue* ~name=abc price<=123))"
        );
    }

    #[test]
    fn test_atom_parser() {
        let p = atom_parser();
        assert_eq!(
            p.parse("  name:abc  ").output(),
            Some(&QueryAST::Atom(
                "name".to_string(),
                OperatorAST::Colon,
                FieldValueAST::Term("abc".to_string())
            ))
        );

        assert_eq!(
            p.parse("  price <= 123  ").output(),
            Some(&QueryAST::Atom(
                "price".to_string(),
                OperatorAST::Le,
                FieldValueAST::Integer(123)
            ))
        );

        assert_eq!(
            p.parse("  price<=123  ").output(),
            Some(&QueryAST::Atom(
                "price".to_string(),
                OperatorAST::Le,
                FieldValueAST::Integer(123)
            ))
        );
    }

    #[test]
    fn test_identifier_parser() {
        let p = identifier_parser();
        assert_eq!(p.parse("abcd").output(), Some(&"abcd".to_string()));
        assert_eq!(p.parse("ab.cd").output(), Some(&"ab.cd".to_string()));
        assert_eq!(p.parse("ab_cd").output(), Some(&"ab_cd".to_string()));
        assert_eq!(p.parse("ab-cd-").output(), Some(&"ab-cd-".to_string()));
        assert_eq!(p.parse("ab-cd-<").output(), None);
    }

    #[test]
    fn test_operator_parser() {
        let p = operator_parser();
        assert_eq!(p.parse(":").output(), Some(&OperatorAST::Colon));
        assert_eq!(p.parse("<").output(), Some(&OperatorAST::Lt));
        assert_eq!(p.parse("> ").output(), Some(&OperatorAST::Gt));
        assert_eq!(p.parse(" =").output(), Some(&OperatorAST::Eq));
        assert_eq!(p.parse("<=  ").output(), Some(&OperatorAST::Le));
        assert_eq!(p.parse("  >=").output(), Some(&OperatorAST::Ge));
        assert_eq!(p.parse(" H3IN ").output(), Some(&OperatorAST::H3Inside));
    }

    #[test]
    fn test_field_value_parser() {
        let parser = field_value_parser();

        assert_eq!(parser.parse("").output(), None);
        assert_eq!(
            parser.parse("abc").output(),
            Some(&FieldValueAST::Term("abc".to_string()))
        );
        assert_eq!(
            parser.parse("abc*").output(),
            Some(&FieldValueAST::Prefix("abc".to_string()))
        );

        assert_eq!(
            parser.parse("\"boudin blanc\"").output(),
            Some(&FieldValueAST::Term("boudin blanc".to_string()))
        );

        assert_eq!(
            parser.parse("\"boudin \\\" blanc\"").output(),
            Some(&FieldValueAST::Term("boudin \" blanc".to_string()))
        );

        assert_eq!(
            parser.parse("\"boudin\\* \\\" blanc\"*").output(),
            Some(&FieldValueAST::Prefix("boudin* \" blanc".to_string()))
        );

        assert!(parser.parse("\"boudin blanc").has_errors());

        assert_eq!(
            parser.parse("\"123\"").output(),
            Some(&FieldValueAST::Term("123".to_string()))
        );
        assert_eq!(
            parser.parse("123").output(),
            Some(&FieldValueAST::Integer(123))
        );
        assert_eq!(
            parser.parse("123*").output(),
            Some(&FieldValueAST::Prefix("123".to_string()))
        );
        assert_eq!(
            parser.parse("-123abc").output(),
            Some(&FieldValueAST::Term("-123abc".to_string()))
        );
    }
}
