use core::fmt;

struct DebugDisplay<T: fmt::Display>(T);

impl<T: fmt::Display> fmt::Debug for DebugDisplay<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(inner) = self;
        fmt::Display::fmt(&inner, f)
    }
}

fn interpret(src: &str) -> Result<String, DebugDisplay<String>> {
    let tokens: Vec<_> = lexer::Lexer::from(src).collect();
    let program = parser::parse_program(&tokens).expect("Failed to parse");
    let (program, _) = ast::convert(&program).expect("Failed to typecheck");
    let mut output: Vec<u8> = vec![];
    let result = crate::interpret(&mut output, &program);
    let mut output = String::from_utf8(output).expect("Somehow got non-UTF8 output");
    match result {
        Ok(()) => Ok(output),
        Err(e) => {
            use fmt::Write;
            writeln!(output, "{e}").expect("Error formatting shouldn't fail");
            Err(DebugDisplay(output))
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
        for_loops => "for_loops"
        function_parameters => "function_parameters"
        function_return => "function_return"
        identifiers => "identifiers"
        lazy_operators => "lazy_operators"
        length => "length"
        logical_operators => "logical_operators"
        nested_control => "nested_control"
        new_array => "new_array"
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
        assert => "assert"
        invalid_new_array_length => "invalid_new_array_length"
        oob_big => "oob_big"
        oob_neg => "oob_neg"
        oob_zero => "oob_zero"
        panic => "panic"
    ]
}
