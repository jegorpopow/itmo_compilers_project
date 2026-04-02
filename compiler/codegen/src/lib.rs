#![expect(dead_code, reason = "WIP")]
#![expect(unreachable_pub, reason = "WIP")]
#![expect(unused_variables, reason = "WIP")]

use crate::bytecode::{Instruction, TypeId};
use ast::IdentifierTable;
use ast::{AnalysisError, AnalysisResult, Block, Expression, LvalueExpression, Statement};

pub mod bytecode;

struct Compiler<'a> {
    identifiers: &'a IdentifierTable,
    bytecode: Vec<Instruction>,
    fresh_label_counter: u64,
}

impl<'a> Compiler<'a> {
    pub fn new(table: &'a IdentifierTable) -> Self {
        Compiler {
            identifiers: table,
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
                    loc: self
                        .identifiers
                        .get_binding(identifier)
                        .ensure_is_var()?
                        .relative_location,
                });
                Ok(())
            }
            LvalueExpression::Member { lhs, member_name } => {
                self.compile_lvalue_expr(lhs)?;
                self.bytecode
                    .push(Instruction::FieldAddress { field_offset: 0 }); // TODO
                Ok(())
            }
            LvalueExpression::Index { lhs, index } => {
                self.compile_expr(index)?;
                self.compile_lvalue_expr(lhs)?;
                self.bytecode.push(Instruction::ElementAddress);
                Ok(())
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expression) -> AnalysisResult<()> {
        match expr {
            Expression::LvalueToRvalue(lvalue_expression) => match &**lvalue_expression {
                LvalueExpression::Identifier(identifier) => {
                    self.bytecode.push(Instruction::Load {
                        loc: self
                            .identifiers
                            .get_binding(identifier)
                            .ensure_is_var()?
                            .relative_location,
                    });
                }
                default @ (LvalueExpression::Member { .. } | LvalueExpression::Index { .. }) => {
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
                    value: i64::from(*bool_literal),
                });
            }
            Expression::Call { callee, args } => {
                for arg in args.iter().rev() {
                    self.compile_expr(arg)?;
                }
                self.bytecode.push(Instruction::Call { function_label: 0 }); // FIXME(!)
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
            Expression::Cast { operand, target } => {
                self.compile_expr(operand)?;
            }
            Expression::New { t, fields } => {
                let effective_type = self.identifiers.get_effective_type(t)?;

                match &*effective_type {
                    ast::Type::Int
                    | ast::Type::Real
                    | ast::Type::Bool
                    | ast::Type::Null
                    | ast::Type::Unit => {
                        return Err(AnalysisError {
                            what: "Unboxed types can not be new-constructed".to_string(),
                        });
                    }
                    ast::Type::Alias(_) => {
                        return Err(AnalysisError {
                            what: "Effective type can not be alias".to_string(),
                        });
                    }
                    ast::Type::Record(record_description) => {
                        self.bytecode.push(Instruction::AllocRecord {
                            type_id: TypeId(0), // TODO: make types interning to build a
                            size: 0,
                        });

                        // Fields initialisation
                        for (name, expr) in fields.clone().unwrap_or_default() {
                            self.bytecode.push(Instruction::Dup);
                            self.bytecode.push(Instruction::IntConst {
                                value: i64::try_from(record_description.get_field_index(&name)?)
                                    .expect("Internal compiler error: to big structure type"),
                            });
                            self.bytecode.push(Instruction::ElementAddress);
                            self.compile_expr(&expr)?;
                            self.bytecode.push(Instruction::StoreAddress);
                        }
                    }
                    ast::Type::Array(array_description) => {
                        if let Some(length) = array_description.length {
                            self.bytecode.push(Instruction::AllocArray {
                                type_id: TypeId(0),
                                size: length
                                    .try_into()
                                    .expect("Internal compiler error, too long array"),
                            });
                        } else {
                            // TODO: Support `array [] T`` type for allocation ???
                            return Err(AnalysisError {
                                what: "Allocation of array of unknown size is not supported"
                                    .to_string(),
                            });
                        }
                    }
                }
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
        for block_elem in &block.elems {
            match block_elem {
                ast::BlockElem::Stmt(statement) => self.compile_statement(statement)?,
                ast::BlockElem::Decl(simple_binding) => match &simple_binding.decl {
                    ast::SimpleDecl::Var(var_decl) => {
                        let intialiser = var_decl
                            .initialiser
                            .clone()
                            .ok_or(AnalysisError {
                                what: "placeholder".to_string(),
                            })
                            .or_else(|_| self.identifiers.get_default_intialiser(&var_decl.t))?;
                        // Local variable intialisation is just a `push`
                        self.compile_expr(&intialiser)?;
                    }

                    ast::SimpleDecl::Type(type_decl) => (),
                },
            }
        }

        // Discard local variables
        self.bytecode
            .push(Instruction::DropMany(block.locals_count));

        Ok(())
    }

    fn compile_statement(&mut self, stmt: &Statement) -> AnalysisResult<()> {
        match stmt {
            Statement::Assignment { lhs, rhs } => match &**lhs {
                LvalueExpression::Identifier(identifier) => {
                    // Microoptimisation: use direct Store instruction instead of calculating address
                    self.compile_expr(rhs)?;
                    self.bytecode.push(Instruction::Store {
                        loc: self
                            .identifiers
                            .get_binding(identifier)
                            .ensure_is_var()?
                            .relative_location,
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
            Statement::For {
                counter,
                lower_bound,
                upper_bound,
                order,
                body,
            } => todo!(),
            Statement::ForEach {
                counter,
                collection,
                order,
                body,
            } => todo!(),
            Statement::Print { value } => {
                self.compile_expr(value)?;
                self.bytecode
                    .push(Instruction::Print { type_id: TypeId(0) }); // TODO: typeid
            }
            Statement::Return { value } => {
                self.compile_expr(value)?;
                self.bytecode.push(Instruction::Ret);
            }
        }

        Ok(())
    }
}
