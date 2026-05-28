use core::convert::Infallible;

#[expect(clippy::unnecessary_wraps, reason = "uniformity in the testing infra")]
fn compile(src: &str) -> Result<String, Infallible> {
    let program = parser::parse_program(lexer::Lexer::from(src)).expect("Failed to parse");
    let program = ast::convert(&program).expect("Failed to typecheck");
    let program = crate::compile(&program);
    Ok(format!("{program:#?}\n"))
}

testing::tests! {
    folder = "codegen"
    extension = "txt"
    fun = compile
    pass = [
        arithmetic_operations => "arithmetic_operations"
        arrays_and_records => "arrays_and_records"
        assert => "assert"
        comparison_operators => "comparison_operators"
        complex_expressions => "complex_expressions"
        conditionals => "conditionals"
        constant => "constant"
        deep_conditionals => "deep_conditionals"
        default_init => "default_init"
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
    ]
}
