use ast::AnalysisResult;

use crate::bytecode::Serialize;

fn compile(src: &str) -> AnalysisResult<String> {
    use core::fmt::Write;

    let program = parser::parse_program(lexer::Lexer::from(src)).expect("Failed to parse");
    let program = ast::convert(&program).expect("Failed to typecheck");
    let code = crate::codegen(&program)?;
    let mut result = String::new();
    writeln!(result, "{code:#?}")
        .and_then(|()| {
            const COLUMNS: usize = 8;
            let mut column = 0;
            code.serialize::<core::fmt::Error>(&mut |bytes| {
                for byte in bytes {
                    if column == COLUMNS {
                        writeln!(result)?;
                        column = 0
                    }
                    if column != 0 {
                        write!(result, " ")?
                    }
                    column += 1;
                    write!(result, "{byte:02X}")?
                }
                Ok(())
            })
        })
        .expect("Formatting should not fail");
    result.push('\n');
    Ok(result)
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
