use lexer::Lexer;

use crate::{FinalError, Parser, Program, parse_program};

/// Sadly, `parse_program` will never finish successfully without reaching EOF.
/// But the public `parse_expr` will. So here's a test to check that behavior.
#[test]
fn no_eof() {
    let src = "(1 + 2) a";
    let mut parser = Parser::new(Lexer::from(src));
    let res = parser.parse_expr();
    let res = parser.finish(res);
    testing::expect![[r#"
        Parsing error at 1:8: expected EOF, but found an identifier.
        However, managed to parse:
        BinOp {
            op: Plus,
            lhs: Literal(
                Integer {
                    repr: "1",
                    value: 1,
                },
            ),
            rhs: Literal(
                Integer {
                    repr: "2",
                    value: 2,
                },
            ),
        }"#]]
    .assert_eq(&res.expect_err("This is not a valid expression").to_string())
}

fn parse(src: &str) -> Result<String, FinalError<Program>> {
    parse_program(Lexer::from(src)).map(|program| format!("{program:#?}\n"))
}

testing::tests! {
    folder = "parser"
    extension = "txt"
    fun = parse
    pass = [
        arithmetic_operations => "arithmetic_operations"
        array_slice_eq => "array_slice_eq"
        arrays_and_records => "arrays_and_records"
        assert => "assert"
        bool_arith => "bool_arith"
        bool_ord => "bool_ord"
        comparison_operators => "comparison_operators"
        complex_expressions => "complex_expressions"
        conditionals => "conditionals"
        constant => "constant"
        deep_conditionals => "deep_conditionals"
        default_init => "default_init"
        field_not_record => "field_not_record"
        for_loops => "for_loops"
        function_parameters => "function_parameters"
        function_return => "function_return"
        identifiers => "identifiers"
        invalid_array_length => "invalid_array_length"
        invalid_new_array_length => "invalid_new_array_length"
        lazy_operators => "lazy_operators"
        length => "length"
        local_type => "local_type"
        logical_operators => "logical_operators"
        nested_control => "nested_control"
        new_array => "new_array"
        new_array_fixed => "new_array_fixed"
        no_such_field => "no_such_field"
        not_scalar_arith => "not_scalar_arith"
        not_scalar_ord => "not_scalar_ord"
        null_array_length => "null_array_length"
        null_record_field => "null_record_field"
        oob_big => "oob_big"
        oob_neg => "oob_neg"
        oob_zero => "oob_zero"
        operator_precedence => "operator_precedence"
        panic => "panic"
        parse_minus => "parse_minus"
        print => "print"
        raytracer => "raytracer"
        real_comparisons => "real_comparisons"
        real_literals => "real_literals"
        real_mod => "real_mod"
        records => "records"
        recursive_function => "recursive_function"
        recursive_types => "recursive_types"
        references => "references"
        routine_redefinition => "routine_redefinition"
        shadow => "shadow"
        type_aliases => "type_aliases"
        type_conversions => "type_conversions"
        variable_declarations => "variable_declarations"
        while_loops => "while_loops"
    ]
    fail = [
        binary_not => "binary_not"
        invalid => "invalid"
        lexer_invalid => "lexer_invalid"
        malformed_integer => "malformed_integer"
    ]
}
