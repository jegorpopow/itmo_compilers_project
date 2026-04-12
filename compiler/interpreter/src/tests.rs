use core::fmt;

fn interpret(src: &str) -> Result<String, String> {
    let program = parser::parse_program(lexer::Lexer::from(src)).expect("Failed to parse");
    let program = ast::convert(&program).expect("Failed to typecheck");
    let mut output: Vec<u8> = vec![];
    let result = crate::interpret(&mut output, &program);
    let mut output = String::from_utf8(output).expect("Somehow got non-UTF8 output");
    match result {
        Ok(()) => Ok(output),
        Err(e) => {
            use fmt::Write;
            writeln!(output, "{e}").expect("Error formatting shouldn't fail");
            Err(output)
        }
    }
}

testing::tests! {
    folder = "run"
    extension = "stdout"
    fun = interpret
    pass = [
        arithmetic_operations => "arithmetic_operations"
        arrays_and_records => "arrays_and_records"
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
        lazy_operators => "lazy_operators"
        length => "length"
        local_type => "local_type"
        logical_operators => "logical_operators"
        nested_control => "nested_control"
        new_array => "new_array"
        operator_precedence => "operator_precedence"
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
        assert => "assert"
        invalid_new_array_length => "invalid_new_array_length"
        oob_big => "oob_big"
        oob_neg => "oob_neg"
        oob_zero => "oob_zero"
        panic => "panic"
    ]
}
