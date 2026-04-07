use lexer::Lexer;

use crate::{ParserError, Program, parse_program};

fn parse(src: &str) -> Result<String, ParserError<Program>> {
    let tokens: Vec<_> = Lexer::from(src).collect();
    parse_program(&tokens).map(|program| format!("{program:#?}"))
}

testing::tests! {
    folder = "parser"
    extension = "txt"
    fun = parse
    pass = [
        arithmetic_operations => "arithmetic_operations"
        arrays_and_records => "arrays_and_records"
        assert => "assert"
        comparison_operators => "comparison_operators"
        complex_expressions => "complex_expressions"
        conditionals => "conditionals"
        constant => "constant"
        deep_conditionals => "deep_conditionals"
        for_loops => "for_loops"
        function_parameters => "function_parameters"
        function_return => "function_return"
        identifiers => "identifiers"
        lazy_operators => "lazy_operators"
        length => "length"
        logical_operators => "logical_operators"
        nested_control => "nested_control"
        new_array => "new_array"
        new_array_fixed => "new_array_fixed"
        oob_big => "oob_big"
        oob_neg => "oob_neg"
        oob_zero => "oob_zero"
        operator_precedence => "operator_precedence"
        panic => "panic"
        parse_minus => "parse_minus"
        raytracer => "raytracer"
        real_comparisons => "real_comparisons"
        real_literals => "real_literals"
        records => "records"
        recursive_function => "recursive_function"
        recursive_types => "recursive_types"
        routine_redefinition => "routine_redefinition"
        shadow => "shadow"
        type_aliases => "type_aliases"
        type_conversions => "type_conversions"
        variable_declarations => "variable_declarations"
        while_loops => "while_loops"
    ]
    fail = [
        invalid => "invalid"
        lexer_invalid => "lexer_invalid"
    ]
}
