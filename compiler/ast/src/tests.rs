use lexer::Lexer;
use parser::parse_program;

use crate::{AnalysisResult, convert};

fn get_ast(src: &str) -> AnalysisResult<String> {
    let tokens: Vec<_> = Lexer::from(src).collect();
    let program = parse_program(&tokens).expect("Failed to parse");
    convert(&program).map(|res| format!("{res:#?}\n"))
}

testing::tests! {
    folder = "ast"
    extension = "txt"
    fun = get_ast
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
        invalid_new_array_length => "invalid_new_array_length"
        lazy_operators => "lazy_operators"
        length => "length"
        local_type => "local_type"
        logical_operators => "logical_operators"
        nested_control => "nested_control"
        new_array => "new_array"
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
        records => "records"
        recursive_function => "recursive_function"
        recursive_types => "recursive_types"
        references => "references"
        shadow => "shadow"
        type_aliases => "type_aliases"
        type_conversions => "type_conversions"
        variable_declarations => "variable_declarations"
        while_loops => "while_loops"
    ]
    fail = [
        array_slice_eq => "array_slice_eq"
        invalid_array_length => "invalid_array_length"
        new_array_fixed => "new_array_fixed"
        routine_redefinition => "routine_redefinition"
    ]
}
