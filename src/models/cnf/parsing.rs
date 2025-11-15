// Parsing CNF queries
use chumsky::prelude::*;

#[derive(Debug, PartialEq)]
enum FieldValue {
    Term(String),
    Prefix(String),
    Integer(i64),
}

fn field_value_parser<'src>() -> impl Parser<'src, &'src str, FieldValue> {
    let term_char = just('\\')
        .ignore_then(any()) // After backslash, accept any character
        .or(none_of('"')); // Or any character that's not a quote, as this is meant to use into a phrase parser.

    let phrase = just('"')
        .ignore_then(term_char.repeated().collect::<String>())
        .then_ignore(just('"'));

    let word = none_of([' ', '\t', '\n', '"', '(', ')', ':', '*'])
        .repeated()
        .at_least(1)
        .collect::<String>();

    choice((phrase, word))
        .then(just('*').or_not())
        .padded()
        .map(|(t, wc)| {
            if wc.is_some() {
                FieldValue::Prefix(t)
            } else {
                FieldValue::Term(t)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_field_value_parser() {
        let parser = field_value_parser();

        assert_eq!(parser.parse("").output(), None);
        assert_eq!(
            parser.parse("abc").output(),
            Some(&FieldValue::Term("abc".to_string()))
        );
        assert_eq!(
            parser.parse("abc*").output(),
            Some(&FieldValue::Prefix("abc".to_string()))
        );

        assert_eq!(
            parser.parse("\"boudin blanc\"").output(),
            Some(&FieldValue::Term("boudin blanc".to_string()))
        );

        assert_eq!(
            parser.parse("\"boudin \\\" blanc\"").output(),
            Some(&FieldValue::Term("boudin \" blanc".to_string()))
        );

        assert_eq!(
            parser.parse("\"boudin\\* \\\" blanc\"*").output(),
            Some(&FieldValue::Prefix("boudin* \" blanc".to_string()))
        );
    }
}
