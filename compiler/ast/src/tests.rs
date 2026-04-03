use lexer::Lexer;
use parser::parse_program;

use crate::{AnalysisResult, convert};

fn get_ast(src: &str) -> AnalysisResult<String> {
    let tokens: Vec<_> = Lexer::from(src).collect();
    let (program, parsing_errors) = parse_program(&tokens).expect("Failed to parse");
    assert!(
        parsing_errors.is_empty(),
        "Parsing errors: {parsing_errors:?}"
    );
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
        deep_conditionals => "deep_conditionals"
        for_loops => "for_loops"
        function_parameters => "function_parameters"
        function_return => "function_return"
        identifiers => "identifiers"
        lazy_operators => "lazy_operators"
        length => "length"
        logical_operators => "logical_operators"
        nested_control => "nested_control"
        oob_big => "oob_big"
        oob_neg => "oob_neg"
        oob_zero => "oob_zero"
        operator_precedence => "operator_precedence"
        parse_minus => "parse_minus"
        raytracer => "raytracer"
        real_comparisons => "real_comparisons"
        real_literals => "real_literals"
        records => "records"
        recursive_function => "recursive_function"
        recursive_types => "recursive_types"
        shadow => "shadow"
        type_aliases => "type_aliases"
        type_conversions => "type_conversions"
        variable_declarations => "variable_declarations"
        while_loops => "while_loops"
    ]
    fail = [
        routine_redefinition => "routine_redefinition"
    ]
}
