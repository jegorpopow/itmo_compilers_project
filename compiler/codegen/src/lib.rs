use std::rc::Rc;

use ast::{
    AnalysisError, AnalysisResult, Binding, Bindings, Block, Decl, Expression, LvalueExpression,
    Program, Routine, RoutineBody, RoutineDecl, Statement, Type,
};
use common::{Integer, Position, RawIdentifier};

pub mod bytecode;

use bytecode::{Instruction, TypeId};

#[derive(Debug)]
pub struct Compiler<'a> {
    bindings: &'a Bindings,
    bytecode: Vec<Instruction>,
    fresh_label_counter: u64,
}

impl<'a> Compiler<'a> {
    #[must_use]
    pub fn new(table: &'a Bindings) -> Self {
        Compiler {
            bindings: table,
            bytecode: Vec::new(),
            fresh_label_counter: 0,
        }
    }

    fn get_fresh_label(&mut self) -> u64 {
        let result = self.fresh_label_counter;
        self.fresh_label_counter += 1;
        result
    }

    fn compile_lvalue_expr(&mut self, expr: &LvalueExpression) -> AnalysisResult<()> {
        match expr {
            LvalueExpression::Identifier(identifier) => {
                self.bytecode.push(Instruction::AddressOf {
                    loc: self.bindings[identifier].ensure_is_var()?.relative_location,
                });
                Ok(())
            }
            LvalueExpression::Member { lhs, member_name } => {
                self.compile_lvalue_expr(lhs)?;
                self.bytecode
                    .push(Instruction::FieldAddress { field_offset: 0 });
                todo!("Compute field offset for {member_name:?}")
            }
            LvalueExpression::Index { lhs, index } => {
                self.compile_expr(index)?;
                self.compile_lvalue_expr(lhs)?;
                self.bytecode.push(Instruction::ElementAddress);
                Ok(())
            }
        }
    }

    fn compiler_new(
        &mut self,
        t: &Rc<Type>,
        fields: &[(RawIdentifier, Rc<Expression>)],
    ) -> AnalysisResult<()> {
        let effective_type = self.bindings.get_effective_type(t)?;

        match &*effective_type {
            Type::Int | Type::Real | Type::Bool | Type::Null | Type::Unit => {
                return Err(AnalysisError {
                    what: "Unboxed types can not be new-constructed".to_string(),
                });
            }
            Type::Alias(_) => {
                return Err(AnalysisError {
                    what: "Effective type can not be alias".to_string(),
                });
            }
            Type::Record(record_description) => {
                self.bytecode.push(Instruction::AllocRecord {
                    type_id: TypeId(0), // TODO: make types interning to build a
                    size: 0,
                });

                // Fields initialisation
                for (name, expr) in fields {
                    self.bytecode.push(Instruction::Dup);
                    self.bytecode.push(Instruction::IntConst {
                        value: Integer::try_from(record_description.get_field_index(name)?)
                            .expect("Internal compiler error: to big structure type"),
                    });
                    self.bytecode.push(Instruction::ElementAddress);
                    self.compile_expr(expr)?;
                    self.bytecode.push(Instruction::StoreAddress);
                }
            }
            Type::Array(array_description) => {
                let Some(length) = array_description.length else {
                    // TODO: Support `array [] T`` type for allocation ???
                    return Err(AnalysisError {
                        what: "Allocation of array of unknown size is not supported".to_string(),
                    });
                };
                self.bytecode.push(Instruction::AllocArray {
                    type_id: TypeId(0),
                    size: length
                        .try_into()
                        .expect("Internal compiler error, too long array"),
                })
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expression) -> AnalysisResult<()> {
        match expr {
            Expression::LvalueToRvalue(lvalue_expression) => match &**lvalue_expression {
                LvalueExpression::Identifier(identifier) => {
                    self.bytecode.push(Instruction::Load {
                        loc: self.bindings[identifier].ensure_is_var()?.relative_location,
                    });
                }
                LvalueExpression::Member { .. } | LvalueExpression::Index { .. } => {
                    self.compile_lvalue_expr(lvalue_expression)?;
                    self.bytecode.push(Instruction::LoadAddress);
                }
            },
            Expression::IntegerLiteral(integer_literal) => {
                self.bytecode.push(Instruction::IntConst {
                    value: integer_literal.value,
                });
            }
            Expression::RealLiteral(real_literal) => {
                self.bytecode.push(Instruction::RealConst {
                    value: real_literal.value,
                });
            }
            Expression::BoolLiteral(bool_literal) => {
                self.bytecode.push(Instruction::IntConst {
                    value: Integer::from(*bool_literal),
                });
            }
            Expression::Call { callee, args } => {
                for arg in args.iter().rev() {
                    self.compile_expr(arg)?;
                }
                self.bytecode.push(Instruction::Call { function_label: 0 });
                todo!("Compute function label for {callee:?}")
            }
            Expression::BinOp { op, lhs, rhs } => {
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                self.bytecode.push(Instruction::BinOp { op: *op });
            }
            Expression::UnOp { op, operand } => {
                self.compile_expr(operand)?;
                self.bytecode.push(Instruction::UnOp { op: *op });
            }
            Expression::Cast { operand, target: _ } => {
                self.compile_expr(operand)?;
            }
            Expression::New { t, fields } => {
                self.compiler_new(t, fields.as_deref().unwrap_or_default())?
            }
            Expression::LengthOf { arr } => {
                self.compile_expr(arr)?;
                self.bytecode.push(Instruction::ArraySize);
            }
            Expression::Null => {
                self.bytecode.push(Instruction::NullConst);
            }
            Expression::IntToBool(expression) => {
                self.compile_expr(expression)?;
                self.bytecode.push(Instruction::IntToBool);
            }
            Expression::BoolToInt(expression) => {
                self.compile_expr(expression)?;
            }
            Expression::RealToInt(expression) => {
                self.compile_expr(expression)?;
                self.bytecode.push(Instruction::RealToInt);
            }
            Expression::IntToReal(expression) => {
                self.compile_expr(expression)?;
                self.bytecode.push(Instruction::IntToReal);
            }
        }

        Ok(())
    }

    fn compile_block(&mut self, block: &Block) -> AnalysisResult<()> {
        for statement in &block.stmts {
            self.compile_statement(statement)?
        }

        // Discard local variables
        self.bytecode
            .push(Instruction::DropMany(block.locals_count));

        Ok(())
    }

    fn compile_statement(&mut self, stmt: &Statement) -> AnalysisResult<()> {
        match stmt {
            &Statement::Panic {
                pos: Position { line, column },
            } => self.bytecode.push(Instruction::Panic {
                code: 1,
                line: line.try_into().expect("Over 4 billion lines of code?"),
                column: column.try_into().expect(
                    "I understand choosing 120 for your line width instead of 80, \
                    but over 65 thousand characters in a single line? Really?",
                ),
            }),
            Statement::Assignment { lhs, rhs } => match &**lhs {
                LvalueExpression::Identifier(identifier) => {
                    // Microoptimisation: use direct Store instruction instead of calculating address
                    self.compile_expr(rhs)?;
                    self.bytecode.push(Instruction::Store {
                        loc: self.bindings[identifier].ensure_is_var()?.relative_location,
                    });
                }
                LvalueExpression::Index { .. } | LvalueExpression::Member { .. } => {
                    self.compile_lvalue_expr(lhs)?;
                    self.compile_expr(rhs)?;
                    self.bytecode.push(Instruction::StoreAddress);
                }
            },
            Statement::While { condition, body } => {
                let body_label = self.get_fresh_label();
                let condition_label = self.get_fresh_label();

                self.bytecode.push(Instruction::Jump {
                    label: condition_label,
                });
                self.bytecode.push(Instruction::Label { id: body_label });
                self.compile_block(body)?;
                self.bytecode.push(Instruction::Label {
                    id: condition_label,
                });
                self.compile_expr(condition)?;
                self.bytecode
                    .push(Instruction::JumpNotZero { label: body_label });
            }
            Statement::Expr(expression) => {
                self.compile_expr(expression)?;
                self.bytecode.push(Instruction::Drop);
            }
            Statement::If {
                condition,
                on_true,
                on_false,
            } => {
                let on_false_label = self.get_fresh_label();
                self.compile_expr(condition)?;
                self.bytecode.push(Instruction::JumpZero {
                    label: on_false_label,
                });
                self.compile_block(on_true)?;
                self.bytecode
                    .push(Instruction::Label { id: on_false_label });
                if let Some(on_false) = on_false {
                    self.compile_block(on_false)?;
                }
            }
            Statement::For { .. } => todo!(),
            Statement::ForEach { .. } => todo!(),
            Statement::Print { value } => {
                self.compile_expr(value)?;
                self.bytecode
                    .push(Instruction::Print { type_id: TypeId(0) }); // TODO: typeid
            }
            Statement::Return { value } => {
                self.compile_expr(value)?;
                self.bytecode.push(Instruction::Ret);
            }

            Statement::Declaration(Binding { name: _, decl }) => match decl {
                ast::LocalDecl::Var(var_decl) => {
                    let initialiser = var_decl
                        .initialiser
                        .clone()
                        .ok_or(AnalysisError {
                            what: "placeholder".to_string(),
                        })
                        .or_else(|_| self.bindings.get_default_initialiser(&var_decl.t))?;
                    // Local variable initialisation is just a `push`
                    self.compile_expr(&initialiser)?;
                }

                ast::LocalDecl::Type(_) | ast::LocalDecl::Const(_) => (),
            },
        }

        Ok(())
    }

    pub fn compile(&mut self, program: &Program) -> AnalysisResult<()> {
        for Binding { name, decl } in &program.0 {
            match decl {
                Decl::Var(v) => todo!("var {name:?} = {v:?}"),
                Decl::Const(v) => todo!("const {name:?} = {v:?}"),
                Decl::Type(t) => todo!("type {name:?} = {t:?}"),
                Decl::Routine(r) => match r {
                    RoutineDecl::Forward { .. } => todo!(),
                    RoutineDecl::Full(Routine {
                        signature,
                        args_bindings,
                        body,
                    }) => {
                        match body {
                            RoutineBody::Block(block) => self.compile_block(block)?,

                            RoutineBody::Expression(expression) => self.compile_expr(expression)?,
                        }
                        todo!(
                            "Do something with signature {signature:?} and args {args_bindings:?}"
                        )
                    }
                },
            }
        }
        Ok(())
    }
}
