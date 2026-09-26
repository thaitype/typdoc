//! Covers SPC-15.
//!
//! A value read by the type its schema gives it. Each expectation is written by hand.

use typdoc_core::{FieldType, Number, Value, coerce};

fn text(written: &str) -> Value {
    Value::Text(written.to_owned())
}

fn number(written: &str) -> Value {
    Value::Number(Number::read(written).expect("a JSON number"))
}

fn coerced(kind: &FieldType, written: &str) -> Option<Value> {
    coerce(kind, &text(written))
}

#[test]
fn a_string_an_enum_and_a_ref_are_their_text_whatever_it_looks_like() {
    for kind in [FieldType::String, FieldType::Enum, FieldType::Ref] {
        for written in [
            "no",
            "yes",
            "on",
            "off",
            "true",
            "1e3",
            "1.10",
            "0755",
            "~",
            "",
            "2026-09-19",
            "WF-3",
        ] {
            assert_eq!(
                coerced(&kind, written),
                Some(text(written)),
                "{kind:?} {written}"
            );
        }
    }
}

#[test]
fn a_number_is_a_json_number_and_nothing_else() {
    for written in ["3", "-3", "0", "0.5", "1.10", "1e3", "1E3", "-2.5e-3"] {
        assert_eq!(
            coerced(&FieldType::Number, written),
            Some(number(written)),
            "{written}"
        );
    }
    for written in [
        "", "abc", "0755", "+3", ".5", "1.", "0x1F", "1_000", " 3", "3 ", "NaN", ".inf", "-.5",
        "1e400", "yes", "true",
    ] {
        assert_eq!(coerced(&FieldType::Number, written), None, "{written:?}");
    }
}

#[test]
fn a_number_keeps_the_kind_of_number_it_was_written_as() {
    let Some(Value::Number(three)) = coerced(&FieldType::Number, "3") else {
        panic!("3 is a number");
    };
    let Some(Value::Number(ratio)) = coerced(&FieldType::Number, "1e3") else {
        panic!("1e3 is a number");
    };

    assert_eq!(three.as_u64(), Some(3));
    assert_eq!(three.converted(), "3");
    assert_eq!(ratio.as_f64(), Some(1000.0));
    assert_eq!(ratio.as_u64(), None);
    assert_eq!(ratio.converted(), "1000.0");
}

#[test]
fn a_bool_is_true_or_false_written_in_lower_case() {
    assert_eq!(coerced(&FieldType::Bool, "true"), Some(Value::Bool(true)));
    assert_eq!(coerced(&FieldType::Bool, "false"), Some(Value::Bool(false)));
    for written in [
        "no", "yes", "on", "off", "y", "n", "True", "TRUE", "False", "FALSE", "1", "0", "", "~",
        "true ",
    ] {
        assert_eq!(coerced(&FieldType::Bool, written), None, "{written:?}");
    }
}

#[test]
fn a_date_is_four_two_and_two_digits_and_a_real_day() {
    for written in ["2026-09-19", "2028-02-29", "0001-01-01", "9999-12-31"] {
        assert_eq!(
            coerced(&FieldType::Date, written),
            Some(Value::Date(written.to_owned())),
            "{written}"
        );
    }
    for written in [
        "2026-9-19",
        "2026-09-9",
        "26-09-19",
        "2026/09/19",
        "2026-02-30",
        "2027-02-29",
        "2026-13-01",
        "2026-00-10",
        "2026-09-19 ",
        "+2026-09-19",
        "2026-09-19T14:30:00+07:00",
        "\u{ff12}\u{ff10}\u{ff12}\u{ff16}-09-19",
        "",
    ] {
        assert_eq!(coerced(&FieldType::Date, written), None, "{written:?}");
    }
}

#[test]
fn a_datetime_has_a_date_an_upper_case_t_seconds_and_an_offset() {
    for written in [
        "2026-09-19T14:30:00+07:00",
        "2026-09-19T14:30:00-03:30",
        "2026-09-19T14:30:00Z",
        "2026-09-19T14:30:00.25+07:00",
        "2028-02-29T23:59:59Z",
    ] {
        assert_eq!(
            coerced(&FieldType::Datetime, written),
            Some(Value::Datetime(written.to_owned())),
            "{written}"
        );
    }
    for written in [
        "2026-09-19",
        "2026-09-19T14:30:00",
        "2026-09-19T14:30+07:00",
        "2026-09-19t14:30:00+07:00",
        "2026-09-19 14:30:00+07:00",
        "2026-09-19T14:30:00+0700",
        "2026-09-19T24:00:00+07:00",
        "2026-02-30T00:00:00Z",
        "2026-09-19T14:30:00 +07:00",
        "",
    ] {
        assert_eq!(coerced(&FieldType::Datetime, written), None, "{written:?}");
    }
}

#[test]
fn a_datetime_with_text_that_is_not_ascii_is_refused() {
    assert_eq!(
        coerced(&FieldType::Datetime, "2026-09-1\u{e9}T14:30:00Z"),
        None
    );
    assert_eq!(
        coerced(&FieldType::Datetime, "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}T"),
        None
    );
}

#[test]
fn a_list_and_a_ref_list_are_their_items_and_only_a_list_fits() {
    let items = Value::List(vec!["a".into(), "b".into()]);
    for kind in [FieldType::List, FieldType::RefList] {
        assert_eq!(coerce(&kind, &items), Some(items.clone()));
        assert_eq!(
            coerce(&kind, &Value::List(Vec::new())),
            Some(Value::List(Vec::new()))
        );
        assert_eq!(coerce(&kind, &text("a")), None);
    }
}

#[test]
fn a_list_does_not_fit_a_scalar_type() {
    let items = Value::List(vec!["3".into()]);
    for kind in [
        FieldType::String,
        FieldType::Number,
        FieldType::Bool,
        FieldType::Date,
        FieldType::Datetime,
        FieldType::Enum,
        FieldType::Ref,
    ] {
        assert_eq!(coerce(&kind, &items), None, "{kind:?}");
    }
}

#[test]
fn a_type_the_format_does_not_name_leaves_a_value_as_written() {
    let kind = FieldType::Other("integer".into());
    let items = Value::List(vec!["a".into()]);

    assert_eq!(coerced(&kind, "3"), Some(text("3")));
    assert_eq!(coerce(&kind, &items), Some(items));
}

#[test]
fn two_numbers_are_equal_when_the_values_they_convert_to_are() {
    assert_eq!(number("1e3"), number("1000.0"));
    assert_eq!(number("1.10"), number("1.1"));
    assert_eq!(
        number("99999999999999999999"),
        number("99999999999999999998")
    );
    assert_ne!(number("3"), number("3.0"));
    assert_ne!(number("3"), number("4"));
}

#[test]
fn a_number_keeps_the_digits_written_and_converts_out_of_them() {
    // Nothing holds 123456789012345678901 exactly: the digits are kept whole, and the converted
    // value is a float to a float's precision.
    for (written, nearest) in [
        ("123456789012345678901", 1.2345678901234568e20),
        ("-9223372036854775809", -9.223372036854776e18),
    ] {
        let Some(Value::Number(read)) = coerced(&FieldType::Number, written) else {
            panic!("{written} is a number");
        };
        assert_eq!(read.written(), written);
        assert!(
            (read.as_f64().unwrap() / nearest - 1.0).abs() < 1e-15,
            "{written}"
        );
    }

    // Up to `u64::MAX` the converted value is an integer, exactly.
    let Some(Value::Number(max)) = coerced(&FieldType::Number, "18446744073709551615") else {
        panic!("u64::MAX is a number");
    };
    assert_eq!(max.written(), "18446744073709551615");
    assert_eq!(max.as_u64(), Some(u64::MAX));

    // The converted value is where two documents whose digits differ by one meet.
    for (written, converted) in [
        ("3", "3"),
        ("1.10", "1.1"),
        ("1e3", "1000.0"),
        ("99999999999999999999", "1e+20"),
        ("99999999999999999998", "1e+20"),
    ] {
        let Some(Value::Number(read)) = coerced(&FieldType::Number, written) else {
            panic!("{written} is a number");
        };
        assert_eq!(read.written(), written, "{written}");
        assert_eq!(read.converted(), converted, "{written}");
    }
}
