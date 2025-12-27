use std::{borrow::Cow, fmt::Display};

// Parsing CNF queries
use chumsky::{container::Seq, prelude::*};
use h3o::CellIndex;
use h3o::{LatLng, Resolution};

use rand::distr::Alphanumeric;
use rand::prelude::IteratorRandom;

use strum::EnumIter;
use strum::IntoEnumIterator;

use crate::models::queries::latlng_within::parse_latlng_within;
use crate::{models::cnf, prelude::CNFQueryable};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum QueryAST {
    Neg(Box<QueryAST>),
    Atom(String, OperatorAST, FieldValueAST),
    And(Box<QueryAST>, Box<QueryAST>),
    Or(Box<QueryAST>, Box<QueryAST>),
}

impl Display for QueryAST {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryAST::Neg(query_ast) => write!(f, "NOT {}", query_ast),
            QueryAST::Atom(field, operator_ast, field_value_ast) => {
                write!(f, "{}{}{}", field, operator_ast, field_value_ast)
            }
            QueryAST::And(query_ast, query_ast1) => {
                write!(f, "( {} AND {} )", query_ast, query_ast1)
            }
            QueryAST::Or(query_ast, query_ast1) => write!(f, "( {} OR {} )", query_ast, query_ast1),
        }
    }
}

fn atom_to_cnf(field: &str, operator: &OperatorAST, field_value: &FieldValueAST) -> cnf::Query {
    match (&operator, &field_value) {
        // A prefix ALWAYS give a prefix, regardless of operator used.
        // It is a bit dirty, but will fix in the future.
        (OperatorAST::H3Inside, FieldValueAST::Term(t)) => t
            .parse::<CellIndex>()
            .map_or_else(|_err| field.has_value(t.clone()), |ci| field.h3in(ci)),
        // Cannot do H3 on integers..
        (OperatorAST::H3Inside, FieldValueAST::Integer(i)) => field.has_value(i.to_string()),

        (OperatorAST::LatLngWithin, FieldValueAST::Term(t)) => parse_latlng_within(t).map_or_else(
            || field.has_value(t.clone()),
            |(ll, radius)| field.latlng_within(ll, radius),
        ),
        // Cannot do LL WITHIN on integers..
        (OperatorAST::LatLngWithin, FieldValueAST::Integer(i)) => field.has_value(i.to_string()),

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
    pub fn to_cnf(&self) -> cnf::Query {
        match &self {
            QueryAST::Neg(query) => !query.to_cnf(),
            QueryAST::Atom(field, operator, field_value) => {
                atom_to_cnf(field, operator, field_value)
            }
            QueryAST::And(query, query1) => query.to_cnf() & query1.to_cnf(),
            QueryAST::Or(query, query1) => query.to_cnf() | query1.to_cnf(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, EnumIter)]
pub(crate) enum OperatorAST {
    Colon,
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
    H3Inside,
    LatLngWithin,
}

impl Display for OperatorAST {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorAST::Colon => write!(f, ":"),
            OperatorAST::Lt => write!(f, "<"),
            OperatorAST::Le => write!(f, "<="),
            OperatorAST::Eq => write!(f, "="),
            OperatorAST::Ge => write!(f, ">="),
            OperatorAST::Gt => write!(f, ">"),
            OperatorAST::H3Inside => write!(f, " H3IN "),
            OperatorAST::LatLngWithin => write!(f, " LLWITHIN "),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum FieldValueAST {
    Term(String),
    Prefix(String),
    Integer(i64),
}

static NON_IDENTIFIERS: [char; 12] = [
    '\\', ' ', '\t', '\n', '"', '(', ')', ':', '*', '<', '>', '=',
];

// Returns the string if it doesnt contain any NON_IDENTIFIERS characters.
// Returns the string with NON_IDENTIFIERS characters escaped with a \ instead.
fn _escape_quote(s: &str) -> Cow<'_, str> {
    match s.contains(NON_IDENTIFIERS) {
        false => Cow::Borrowed(s),
        true => {
            // 2. We found a special character. We must allocate a new String.
            // Estimate capacity: original length + a few extra bytes for backslashes.
            let mut output = String::with_capacity(s.len() + 5);
            output.push('"');

            // We escape only " and \
            // Iterate through the remainder of the string starting at the bad character
            for c in s.chars() {
                if ['"', '\\'].contains(&c) {
                    output.push('\\');
                }
                output.push(c);
            }
            output.push('"');
            Cow::Owned(output)
        }
    }
}

impl Display for FieldValueAST {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValueAST::Term(s) => write!(f, "{}", _escape_quote(s)),
            FieldValueAST::Prefix(s) => write!(f, "{}*", _escape_quote(s)),
            FieldValueAST::Integer(i) => write!(f, "{}", i),
        }
    }
}

pub(crate) fn random_query<T: rand::Rng>(rng: &mut T, max_depth: usize) -> QueryAST {
    match (rng.random_range(0..4), max_depth) {
        (_, 0) => _random_atom(rng), // Reached max depth. do not go deeper.
        (0, _) => QueryAST::Neg(Box::new(random_query(rng, max_depth - 1))),
        (1, _) => _random_atom(rng),
        (2, _) => QueryAST::And(
            Box::new(random_query(rng, max_depth - 1)),
            Box::new(random_query(rng, max_depth - 1)),
        ),
        (3, _) => QueryAST::Or(
            Box::new(random_query(rng, max_depth - 1)),
            Box::new(random_query(rng, max_depth - 1)),
        ),
        (_, _) => unimplemented!(),
    }
}

type MyParseError<'src> = extra::Err<Rich<'src, char>>;

pub(crate) fn query_parser<'src>() -> impl Parser<'src, &'src str, QueryAST, MyParseError<'src>> {
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

fn _random_h3cell<T: rand::Rng>(rng: &mut T) -> h3o::CellIndex {
    // 1. Generate a random Longitude: [-180, 180]
    let lng_deg = rng.random_range(-180.0..180.0);

    // 2. Generate a random Latitude: [-90, 90]
    // NOTE: To get a truly uniform distribution on the sphere,
    // we use a sine-weighted distribution for latitude (asin).
    let lat_deg = rng.random_range::<f64, _>(-1.0..1.0).asin().to_degrees();

    // 3. Convert to H3 Cell
    let coord = LatLng::new(lat_deg, lng_deg).expect("Valid coordinates");
    coord.to_cell(Resolution::Nine)
}

fn _random_atom<T: rand::Rng>(rng: &mut T) -> QueryAST {
    let op = _random_operator(rng);
    let be_correct = rng.random_bool(0.95);
    match (op, be_correct) {
        (op, false) => QueryAST::Atom(_random_identifier(rng), op, _random_field_value(rng)),
        (OperatorAST::Colon, true) => QueryAST::Atom(
            _random_identifier(rng),
            OperatorAST::Colon,
            _random_field_value(rng),
        ),
        (OperatorAST::H3Inside, true) => QueryAST::Atom(
            _random_identifier(rng),
            OperatorAST::H3Inside,
            FieldValueAST::Term(_random_h3cell(rng).to_string()),
        ),
        (OperatorAST::LatLngWithin, true) => {
            let ll = LatLng::from(_random_h3cell(rng));
            let distance = rng.random_range::<u64, _>(0..10_000_000);

            QueryAST::Atom(
                _random_identifier(rng),
                OperatorAST::LatLngWithin,
                FieldValueAST::Term(format!("{},{},{}", ll.lat(), ll.lng(), distance)),
            )
        }
        // all these other ones are comparison things..
        (op, true) => QueryAST::Atom(_random_identifier(rng), op, _random_field_int_value(rng)),
    }
}

fn atom_parser<'src>() -> impl Parser<'src, &'src str, QueryAST, MyParseError<'src>> {
    identifier_parser()
        .then(operator_parser())
        .then(field_value_parser())
        .map(|((s, o), v)| QueryAST::Atom(s, o, v))
        .padded()
}

fn _random_operator<T: rand::Rng>(rng: &mut T) -> OperatorAST {
    OperatorAST::iter().choose(rng).unwrap()
}
fn operator_parser<'src>() -> impl Parser<'src, &'src str, OperatorAST, MyParseError<'src>> {
    choice((
        just(':').to(OperatorAST::Colon),
        just("H3IN").to(OperatorAST::H3Inside),
        just("LLWITHIN").to(OperatorAST::LatLngWithin),
        just("<=").to(OperatorAST::Le),
        just(">=").to(OperatorAST::Ge),
        just('<').to(OperatorAST::Lt),
        just('>').to(OperatorAST::Gt),
        just('=').to(OperatorAST::Eq),
    ))
    .padded()
}

static RESERVED_WORDS: [&str; 5] = ["AND", "OR", "NOT", "H3IN", "LLWITHIN"];

fn _random_identifier<T: rand::Rng>(rng: &mut T) -> String {
    // Pick a random value between 1 and 20
    let len = rng.random_range(2..20);
    let s = (0..len)
        .map(|_| rng.sample(Alphanumeric))
        .map(char::from)
        .collect::<String>();
    if RESERVED_WORDS.contains(&s.as_str()) {
        format!("FIELD_{}", s)
    } else {
        s
    }
}

fn _random_messy_string<T: rand::Rng>(rng: &mut T) -> String {
    let len = rng.random_range(1..20);
    let dist = rand::distr::Uniform::new_inclusive(32u8, 126u8).unwrap();
    (0..len)
        .map(|_| rng.sample(dist))
        .map(char::from)
        .collect::<String>()
}

fn identifier_parser<'src>() -> impl Parser<'src, &'src str, String, MyParseError<'src>> {
    none_of(NON_IDENTIFIERS)
        .filter(|c: &char| !c.is_whitespace())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .padded()
}

fn _random_field_int_value<T: rand::Rng>(rng: &mut T) -> FieldValueAST {
    FieldValueAST::Integer(rng.random_range(-1000..1000))
}

fn _random_field_value<T: rand::Rng>(rng: &mut T) -> FieldValueAST {
    match rng.random_range(0..3) {
        0 => FieldValueAST::Term(_random_messy_string(rng)),
        1 => FieldValueAST::Prefix(_random_messy_string(rng)),
        2 => _random_field_int_value(rng),
        _ => unimplemented!(), // This is never hit
    }
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
    fn test_random_queries() {
        let mut rng = rand::rng();
        for _ in 0..5000 {
            let q = random_query(&mut rng, 3);
            // Check we can parse the string representation of it.
            let s = q.to_string();
            println!("{}", &s);

            let p = query_parser();
            let pres = p.parse(&s);
            assert!(pres.has_output());
            println!("{}", pres.output().unwrap().to_cnf())
        }
    }

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

        // This rounds trips with the string representation of the query.
        assert_eq!(
            p.parse("( NOT ( colour:blue* AND name:abc ) OR price<=123 )")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR ~colour=blue* ~name=abc price<=123))"
        );
        assert_eq!(
            p.parse("NOT (colour:blue* AND name:abc) OR price<=123")
                .output()
                .unwrap()
                .to_string(),
            "( NOT ( colour:blue* AND name:abc ) OR price<=123 )"
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
            p.parse("  name:abc  ").output().unwrap().to_string(),
            "name:abc"
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
            p.parse(" price <= 123  ").output().unwrap().to_string(),
            "price<=123"
        );

        assert_eq!(
            p.parse("  price<=123  ").output(),
            Some(&QueryAST::Atom(
                "price".to_string(),
                OperatorAST::Le,
                FieldValueAST::Integer(123)
            ))
        );

        assert_eq!(
            p.parse(" price<=123  ").output().unwrap().to_string(),
            "price<=123"
        );
    }

    #[test]
    fn test_identifier_parser() {
        let p = identifier_parser();
        assert_eq!(p.parse("abcd").output(), Some(&"abcd".to_string()));
        assert_eq!(p.parse("abcd").output().unwrap().to_string(), "abcd");

        assert_eq!(p.parse("ab.cd").output(), Some(&"ab.cd".to_string()));
        assert_eq!(p.parse("ab.cd").output().unwrap().to_string(), "ab.cd");

        assert_eq!(p.parse("ab_cd").output(), Some(&"ab_cd".to_string()));
        assert_eq!(p.parse("ab_cd").output().unwrap().to_string(), "ab_cd");
        assert_eq!(p.parse("ab-cd-").output(), Some(&"ab-cd-".to_string()));
        assert_eq!(p.parse("ab_cd-").output().unwrap().to_string(), "ab_cd-");

        assert_eq!(p.parse("ab-cd-<").output(), None);

        // Test a 100 times the random generation
        let mut rng = rand::rng();
        for _ in 0..100 {
            let id = _random_identifier(&mut rng);
            let p = identifier_parser();
            assert_eq!(p.parse(&id).output(), Some(&id));
        }
    }

    #[test]
    fn test_operator_parser() {
        let p = operator_parser();
        assert_eq!(p.parse(":").output(), Some(&OperatorAST::Colon));
        assert_eq!(p.parse(":").output().unwrap().to_string(), ":");
        assert_eq!(p.parse("<").output(), Some(&OperatorAST::Lt));
        assert_eq!(p.parse("<").output().unwrap().to_string(), "<");
        assert_eq!(p.parse("> ").output(), Some(&OperatorAST::Gt));
        assert_eq!(p.parse("> ").output().unwrap().to_string(), ">");
        assert_eq!(p.parse(" =").output(), Some(&OperatorAST::Eq));
        assert_eq!(p.parse(" =").output().unwrap().to_string(), "=");

        assert_eq!(p.parse("<=  ").output(), Some(&OperatorAST::Le));
        assert_eq!(p.parse("<= ").output().unwrap().to_string(), "<=");

        assert_eq!(p.parse("  >=").output(), Some(&OperatorAST::Ge));
        assert_eq!(p.parse("  >=").output().unwrap().to_string(), ">=");

        assert_eq!(p.parse("  H3IN   ").output(), Some(&OperatorAST::H3Inside));
        assert_eq!(p.parse("  H3IN   ").output().unwrap().to_string(), " H3IN ");
    }

    #[test]
    fn test_field_value_parser() {
        let parser = field_value_parser();

        assert_eq!(parser.parse("").output(), None);
        assert_eq!(
            parser.parse("abc").output(),
            Some(&FieldValueAST::Term("abc".to_string()))
        );
        assert_eq!(parser.parse("abc").output().unwrap().to_string(), "abc");

        assert_eq!(
            parser.parse("abc*").output(),
            Some(&FieldValueAST::Prefix("abc".to_string()))
        );

        assert_eq!(parser.parse("abc*").output().unwrap().to_string(), "abc*");

        assert_eq!(
            parser.parse("\"boudin blanc\"").output(),
            Some(&FieldValueAST::Term("boudin blanc".to_string()))
        );

        assert_eq!(
            parser
                .parse("\"boudin blanc\"")
                .output()
                .unwrap()
                .to_string(),
            "\"boudin blanc\""
        );

        assert_eq!(
            parser.parse("\"boudin \\\" blanc\"").output(),
            Some(&FieldValueAST::Term("boudin \" blanc".to_string()))
        );
        assert_eq!(
            parser
                .parse("\"boudin \\\" blanc\"")
                .output()
                .unwrap()
                .to_string(),
            "\"boudin \\\" blanc\""
        );

        assert_eq!(
            parser.parse("\"boudin* \\\" blanc\"*").output(),
            Some(&FieldValueAST::Prefix("boudin* \" blanc".to_string()))
        );
        assert_eq!(
            parser
                .parse("\"boudin* \\\" blanc\"*")
                .output()
                .unwrap()
                .to_string(),
            "\"boudin* \\\" blanc\"*"
        );

        assert!(parser.parse("\"boudin blanc").has_errors());

        assert_eq!(
            parser.parse("\"123\"").output(),
            Some(&FieldValueAST::Term("123".to_string()))
        );
        assert_eq!(parser.parse("\"123\"").output().unwrap().to_string(), "123");

        assert_eq!(
            parser.parse("123").output(),
            Some(&FieldValueAST::Integer(123))
        );
        assert_eq!(parser.parse("123").output().unwrap().to_string(), "123");

        assert_eq!(
            parser.parse("123*").output(),
            Some(&FieldValueAST::Prefix("123".to_string()))
        );
        assert_eq!(parser.parse("123*").output().unwrap().to_string(), "123*");

        assert_eq!(
            parser.parse("-123abc").output(),
            Some(&FieldValueAST::Term("-123abc".to_string()))
        );
        assert_eq!(
            parser.parse("-123abc").output().unwrap().to_string(),
            "-123abc"
        );
    }
}
#[cfg(test)]
mod tests_parsing {
    use super::*;

    #[test]
    fn test_escape_quote() {
        assert_eq!(_escape_quote("abc"), Cow::Borrowed("abc"));
        assert_eq!(
            _escape_quote("abc\"def"),
            Cow::Owned::<str>("\"abc\\\"def\"".into())
        );
        assert_eq!(
            _escape_quote("abc\\def"),
            Cow::Owned::<str>("\"abc\\\\def\"".into())
        );
        assert_eq!(
            _escape_quote("abc def"),
            Cow::Owned::<str>("\"abc def\"".into())
        );
        assert_eq!(
            _escape_quote("abc:def"),
            Cow::Owned::<str>("\"abc:def\"".into())
        );
    }

    #[test]
    fn test_query_ast_display() {
        // Atom
        let atom = QueryAST::Atom(
            "f".into(),
            OperatorAST::Colon,
            FieldValueAST::Term("v".into()),
        );
        assert_eq!(format!("{}", atom), "f:v");

        // Neg
        let neg = QueryAST::Neg(Box::new(atom.clone()));
        assert_eq!(format!("{}", neg), "NOT f:v");

        // And
        let and = QueryAST::And(Box::new(atom.clone()), Box::new(neg.clone()));
        assert_eq!(format!("{}", and), "( f:v AND NOT f:v )");

        // Or
        let or = QueryAST::Or(Box::new(atom.clone()), Box::new(neg.clone()));
        assert_eq!(format!("{}", or), "( f:v OR NOT f:v )");
    }

    #[test]
    fn test_operator_ast_display() {
        assert_eq!(format!("{}", OperatorAST::Colon), ":");
        assert_eq!(format!("{}", OperatorAST::Lt), "<");
        assert_eq!(format!("{}", OperatorAST::Le), "<=");
        assert_eq!(format!("{}", OperatorAST::Eq), "=");
        assert_eq!(format!("{}", OperatorAST::Ge), ">=");
        assert_eq!(format!("{}", OperatorAST::Gt), ">");
        assert_eq!(format!("{}", OperatorAST::H3Inside), " H3IN ");
    }

    #[test]
    fn test_field_value_ast_display() {
        assert_eq!(format!("{}", FieldValueAST::Term("v".into())), "v");
        assert_eq!(
            format!("{}", FieldValueAST::Term("v space".into())),
            "\"v space\""
        );
        assert_eq!(format!("{}", FieldValueAST::Prefix("p".into())), "p*");
        assert_eq!(
            format!("{}", FieldValueAST::Prefix("p space".into())),
            "\"p space\"*"
        );
        assert_eq!(format!("{}", FieldValueAST::Integer(42)), "42");
    }

    #[test]
    fn test_atom_to_cnf() {
        // Term
        let cnf = atom_to_cnf("f", &OperatorAST::Colon, &FieldValueAST::Term("v".into()));
        assert_eq!(cnf.to_string(), "(AND (OR f=v))");

        // Prefix
        let cnf = atom_to_cnf("f", &OperatorAST::Colon, &FieldValueAST::Prefix("p".into()));
        assert_eq!(cnf.to_string(), "(AND (OR f=p*))");

        // Int
        let cnf = atom_to_cnf("f", &OperatorAST::Lt, &FieldValueAST::Integer(10));
        assert_eq!(cnf.to_string(), "(AND (OR f<10))");

        let cnf = atom_to_cnf("f", &OperatorAST::Le, &FieldValueAST::Integer(10));
        assert_eq!(cnf.to_string(), "(AND (OR f<=10))");

        let cnf = atom_to_cnf("f", &OperatorAST::Eq, &FieldValueAST::Integer(10));
        assert_eq!(cnf.to_string(), "(AND (OR f==10))");

        let cnf = atom_to_cnf("f", &OperatorAST::Ge, &FieldValueAST::Integer(10));
        assert_eq!(cnf.to_string(), "(AND (OR f>=10))");

        let cnf = atom_to_cnf("f", &OperatorAST::Gt, &FieldValueAST::Integer(10));
        assert_eq!(cnf.to_string(), "(AND (OR f>10))");

        // H3
        let cell = "87194d106ffffff";
        let cnf = atom_to_cnf(
            "f",
            &OperatorAST::H3Inside,
            &FieldValueAST::Term(cell.into()),
        );
        assert_eq!(cnf.to_string(), format!("(AND (OR f=H3IN={}))", cell));

        // H3 with invalid cell -> Term
        let cnf = atom_to_cnf(
            "f",
            &OperatorAST::H3Inside,
            &FieldValueAST::Term("invalid".into()),
        );
        assert_eq!(cnf.to_string(), "(AND (OR f=invalid))");

        // Fallback int with colon
        let cnf = atom_to_cnf("f", &OperatorAST::Colon, &FieldValueAST::Integer(123));
        assert_eq!(cnf.to_string(), "(AND (OR f=123))");
    }

    #[test]
    fn test_random_generators_coverage() {
        let mut rng = rand::rng();

        // _random_h3cell
        let _ = _random_h3cell(&mut rng);

        // _random_atom
        let _ = _random_atom(&mut rng);

        // _random_operator
        let _ = _random_operator(&mut rng);

        // _random_identifier
        let _ = _random_identifier(&mut rng);

        // _random_messy_string
        let _ = _random_messy_string(&mut rng);

        // _random_field_int_value
        let _ = _random_field_int_value(&mut rng);

        // _random_field_value
        let _ = _random_field_value(&mut rng);

        // random_query
        let _ = random_query(&mut rng, 2);
    }
}
#[cfg(test)]
mod tests_parsing_random {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_random_identifier_variety() {
        let mut rng = rand::rng();
        let mut seen = HashSet::new();
        for _ in 0..100 {
            let id = _random_identifier(&mut rng);
            seen.insert(id);
        }
        // Ensure we got at least 50 different identifiers
        assert!(seen.len() > 50);
        // Ensure it's not just "xyzzy"
        assert!(!seen.contains("xyzzy"));
    }

    #[test]
    fn test_random_messy_string_variety() {
        let mut rng = rand::rng();
        let mut seen = HashSet::new();
        for _ in 0..100 {
            let s = _random_messy_string(&mut rng);
            seen.insert(s);
        }
        assert!(seen.len() > 50);
        assert!(!seen.contains("xyzzy"));
    }

    #[test]
    fn test_random_query_recursion() {
        // Testing recursion by checking if we get deeper queries
        let mut rng = rand::rng();
        // Depth 3 should produce something deeper than atom
        let _ = random_query(&mut rng, 3);
        // It's probabilistic, but usually we should get nested structure.
        // We can inspect the structure to verify it's not always simple atom.
        // We can assert that with enough tries, we get a mix of structure types.

        let mut got_and = false;
        let mut got_or = false;
        let mut got_neg = false;

        for _ in 0..100 {
            let q = random_query(&mut rng, 3);
            match q {
                QueryAST::And(_, _) => got_and = true,
                QueryAST::Or(_, _) => got_or = true,
                QueryAST::Neg(_) => got_neg = true,
                _ => {}
            }
        }
        assert!(got_and);
        assert!(got_or);
        assert!(got_neg);
    }
}
