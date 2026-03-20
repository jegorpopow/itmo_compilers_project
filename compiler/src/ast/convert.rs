#![allow(dead_code)]

use crate::ast;
use crate::ast::types::infer_binary_operator_type;
use crate::bytecode::Location;
use crate::operators::{SemanticUnaryOperator, SyntacticOperator};
use crate::parse_tree as pt;

use crate::ast::error::{AnalysisError, AnalysisResult};
use crate::ast::tree::{IdentifierTable, OptionalDecl, cast_to};
use crate::identifier::{Identifier, RawIdentifier};

use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
struct ScopeBlock {
    binders: HashMap<RawIdentifier, usize>,
    locals_in_block: usize,
}

impl ScopeBlock {
    fn new() -> Self {
        ScopeBlock {
            binders: HashMap::new(),
            locals_in_block: 0,
        }
    }

    fn lookup(&self, name: &RawIdentifier) -> Option<usize> {
        self.binders.get(name).copied()
    }

    fn bind(&mut self, name: &RawIdentifier, ident: &Identifier) {
        let _: Option<usize> = self.binders.insert(name.clone(), ident.id);
    }
}

#[derive(Debug)]
struct RoutinePrototype {
    name: RawIdentifier,
    args: Vec<Rc<ast::types::Type>>,
    return_type: Rc<ast::types::Type>,
}

#[derive(Debug)]
struct Converter {
    ident_table: IdentifierTable,
    current_scope: Vec<ScopeBlock>,
    global_count: usize,
    local_count: usize,
    current_routine: Option<RoutinePrototype>,
}

impl Converter {
    pub fn new() -> Self {
        Converter {
            ident_table: IdentifierTable::new(),
            current_scope: vec![ScopeBlock::new()],
            global_count: 0,
            local_count: 0,
            current_routine: None,
        }
    }

    pub fn extract_table(self) -> IdentifierTable {
        self.ident_table
    }

    fn enter_block(&mut self) {
        self.current_scope.push(ScopeBlock::new())
    }

    fn leave_block(&mut self) {
        assert!(self.current_scope.len() > 1);
        self.local_count -= self
            .current_scope
            .last()
            .expect("At least global context is always present")
            .locals_in_block;
        drop(self.current_scope.pop())
    }

    fn get_fresh_global_location(&mut self) -> Location {
        self.global_count += 1;
        Location::Global(u16::try_from(self.global_count - 1).expect("Too many global variables"))
    }

    fn get_fresh_local_location(&mut self) -> Location {
        assert!(
            self.current_scope.len() > 1,
            "Local name binding in global context"
        );

        self.local_count += 1;
        self.current_scope
            .last_mut()
            .expect("At least global context is always present")
            .locals_in_block += 1;
        Location::Local(
            u16::try_from(self.local_count - 1).expect("Too many local variables in function"),
        )
    }

    pub fn bind_global_decl(&mut self, name: &RawIdentifier, decl: ast::tree::Decl) -> Identifier {
        // TODO: process function forward declaration
        let ident = self.ident_table.create_binding(name, decl);
        self.current_scope[0].bind(name, &ident);

        ident
    }

    pub fn rebind_decl(&mut self, ident: &Identifier, new_decl: ast::tree::Decl) {
        self.ident_table.rebind(ident, new_decl);
    }

    pub fn bind_local_decl(&mut self, name: &RawIdentifier, decl: ast::tree::Decl) -> Identifier {
        assert!(
            self.current_scope.len() > 1,
            "Local name binding in global context"
        );

        let ident = self.ident_table.create_binding(name, decl);
        self.current_scope
            .last_mut()
            .expect("At least global context is always present")
            .bind(name, &ident);

        ident
    }

    pub fn bind_decl(
        &mut self,
        is_global: bool,
        name: &RawIdentifier,
        decl: ast::tree::Decl,
    ) -> Identifier {
        if is_global {
            self.bind_global_decl(name, decl)
        } else {
            self.bind_local_decl(name, decl)
        }
    }

    pub fn bind_routine(
        &mut self,
        routine_name: &RawIdentifier,
        decl: ast::tree::RoutineDecl,
    ) -> AnalysisResult<Identifier> {
        let existing_binding = self.lookup(routine_name).cloned();

        match existing_binding {
            Ok(ast::tree::Binding {
                name: ident,
                decl: ast::tree::Decl::Routine(existing_decl),
            }) => {
                if existing_decl.signature() != decl.signature() {
                    Err(AnalysisError {
                        what: format!(
                            "Conflicting signature for declarations of routine {routine_name:?}"
                        ),
                    })
                } else if existing_decl.is_full() && decl.is_full() {
                    Err(AnalysisError {
                        what: format!("Conflicting declarations of routine {routine_name:?}"),
                    })
                } else if existing_decl.is_forward() && decl.is_full() {
                    self.rebind_decl(&ident, ast::tree::Decl::Routine(decl));
                    Ok(ident)
                } else {
                    Ok(ident)
                }
            }
            Ok(ast::tree::Binding { .. }) | Err(_) => {
                // function just shadows previous global variable with the same name
                let ident = self.bind_global_decl(routine_name, ast::tree::Decl::Routine(decl));
                Ok(ident)
            }
        }
    }

    fn lookup<'a>(&'a self, name: &RawIdentifier) -> AnalysisResult<&'a ast::tree::Binding> {
        self.current_scope
            .iter()
            .rev()
            .find_map(|scope_block| scope_block.lookup(name))
            .map(|id| self.ident_table.get_binding_by_id(id))
            .ok_or(AnalysisError {
                what: format!("Unknown name `{name}`"),
            })
    }

    fn convert_type(&self, t: &Rc<pt::types::Type>) -> AnalysisResult<Rc<ast::types::Type>> {
        match &**t {
            pt::types::Type::Int => Ok(Rc::new(ast::types::Type::Int)),
            pt::types::Type::Real => Ok(Rc::new(ast::types::Type::Real)),
            pt::types::Type::Bool => Ok(Rc::new(ast::types::Type::Bool)),
            pt::types::Type::Alias(raw_identifier) => {
                self.lookup(raw_identifier).and_then(|decl| {
                    decl.ensure_is_type()
                        .map(|_| Rc::new(ast::types::Type::Alias(decl.name.clone())))
                })
            }
            pt::types::Type::Record(pt::types::RecordDescription { fields }) => fields
                .iter()
                .map(|raw_desc| {
                    self.convert_type(&raw_desc.t).map(|converted_type| {
                        ast::types::FieldDescription {
                            name: raw_desc.name.clone(),
                            t: converted_type,
                        }
                    })
                })
                .collect::<AnalysisResult<Vec<ast::types::FieldDescription>>>()
                .map(|converted_fields| {
                    Rc::new(ast::types::Type::Record(ast::types::RecordDescription {
                        fields: converted_fields,
                    }))
                }),
            pt::types::Type::Array(pt::types::ArrayDescription { t, length }) => {
                let element_type = self.convert_type(t)?;

                match length {
                    Some(expr) => {
                        let (converted_expr, expr_type) = self.convert_expr(expr)?;
                        drop(expr_type.ensure_is(ast::types::Type::Int)?);
                        let expr_value = converted_expr.try_constexpr_evaluate()?;
                        let length = expr_value.as_usize()?;

                        Ok(Rc::new(ast::types::Type::Array(
                            ast::types::ArrayDescription {
                                t: element_type,
                                length: Some(length),
                            },
                        )))
                    }
                    None => Ok(Rc::new(ast::types::Type::Array(
                        ast::types::ArrayDescription {
                            t: element_type,
                            length: None,
                        },
                    ))),
                }
            }
        }
    }

    fn convert_lvalue_expr(
        &self,
        tree: &Rc<pt::tree::LvalueExpression>,
    ) -> AnalysisResult<(Rc<ast::tree::LvalueExpression>, Rc<ast::types::Type>)> {
        match &**tree {
            pt::tree::LvalueExpression::Identifier(raw_identifier) => {
                match self.lookup(raw_identifier)? {
                    ast::tree::Binding {
                        name,
                        decl: ast::tree::Decl::Var(ast::tree::VarDecl { t, .. }),
                    } => Ok((
                        Rc::new(ast::tree::LvalueExpression::Identifier(name.clone())),
                        Rc::clone(t),
                    )),
                    t => Err(AnalysisError {
                        what: format!(
                            "{:?} is not a name of variable in lvalue exprerssion",
                            t.name
                        ),
                    }),
                }
            }
            pt::tree::LvalueExpression::Member { lhs, member_name } => {
                let (converted_lhs, t) = self.convert_lvalue_expr(lhs)?;
                let effective_lhs_type = self.ident_table.get_effective_type(&t)?;
                let member_type = effective_lhs_type.get_field_type(member_name)?;

                Ok((
                    Rc::new(ast::tree::LvalueExpression::Member {
                        lhs: converted_lhs,
                        member_name: member_name.clone(),
                    }),
                    member_type,
                ))
            }
            pt::tree::LvalueExpression::Index { lhs, index } => {
                let (converted_lhs, lhs_t) = self.convert_lvalue_expr(lhs)?;
                let (converted_index, rhs_t) = self.convert_expr(index)?;
                drop(rhs_t.ensure_is(ast::types::Type::Int)?);
                let effective_lhs_type = self.ident_table.get_effective_type(&lhs_t)?;
                let resulting_type = effective_lhs_type.get_element_type()?;

                Ok((
                    Rc::new(ast::tree::LvalueExpression::Index {
                        lhs: converted_lhs,
                        index: converted_index,
                    }),
                    resulting_type,
                ))
            }
        }
    }

    pub fn convert_expr(
        &self,
        tree: &Rc<pt::tree::Expression>,
    ) -> AnalysisResult<(Rc<ast::tree::Expression>, Rc<ast::types::Type>)> {
        match &**tree {
            pt::tree::Expression::LvalueToRvalue(lvalue_expression) => match &**lvalue_expression {
                pt::tree::LvalueExpression::Member { lhs, member_name }
                    if member_name.name == "length" =>
                {
                    let (lhs, lhs_type) = self.convert_lvalue_expr(lhs)?;

                    if lhs_type.get_element_type().is_ok() {
                        // Length of array
                        Ok((
                            Rc::new(ast::tree::Expression::LenghtOf {
                                arr: Rc::new(ast::tree::Expression::LvalueToRvalue(lhs)),
                            }),
                            Rc::new(ast::types::Type::Int),
                        ))
                    } else {
                        // Just field named `length`
                        self.convert_lvalue_expr(lvalue_expression)
                            .map(|(lvalue, t)| {
                                (Rc::new(ast::tree::Expression::LvalueToRvalue(lvalue)), t)
                            })
                    }
                }
                pt::tree::LvalueExpression::Member { .. }
                | pt::tree::LvalueExpression::Index { .. }
                | pt::tree::LvalueExpression::Identifier(_) => self
                    .convert_lvalue_expr(lvalue_expression)
                    .map(|(lvalue, t)| (Rc::new(ast::tree::Expression::LvalueToRvalue(lvalue)), t)),
            },
            pt::tree::Expression::IntegerLiteral(integer_literal) => Ok((
                Rc::new(ast::tree::Expression::IntegerLiteral(
                    ast::tree::IntegerLiteral {
                        repr: integer_literal.repr.clone(),
                        value: integer_literal.value,
                    },
                )),
                Rc::new(ast::types::Type::Int),
            )),
            pt::tree::Expression::RealLiteral(real_literal) => Ok((
                Rc::new(ast::tree::Expression::RealLiteral(ast::tree::RealLiteral {
                    repr: real_literal.repr.clone(),
                    value: real_literal.value,
                })),
                Rc::new(ast::types::Type::Real),
            )),
            pt::tree::Expression::BoolLiteral(bool_literal) => Ok((
                Rc::new(ast::tree::Expression::BoolLiteral(match bool_literal {
                    pt::tree::BoolLiteral::True => ast::tree::BoolLiteral::True,
                    pt::tree::BoolLiteral::False => ast::tree::BoolLiteral::False,
                })),
                Rc::new(ast::types::Type::Bool),
            )),
            pt::tree::Expression::Call { callee, args } => {
                let ast::tree::RoutineSignature {
                    args: formal_args,
                    return_type,
                } = self.lookup(callee)?.ensure_is_routine()?.signature();
                let arguments_types = formal_args
                    .iter()
                    .map(|(_, arg_type)| Rc::clone(arg_type))
                    .collect::<Vec<Rc<ast::types::Type>>>();

                let converted_expressions = args
                    .iter()
                    .map(|arg| self.convert_expr(arg))
                    .collect::<AnalysisResult<
                    Vec<(Rc<ast::tree::Expression>, Rc<ast::types::Type>)>,
                >>()?;

                if arguments_types.len() != converted_expressions.len() {
                    Err(AnalysisError {
                        what: format!(
                            "routine `{callee}` expexts {} arguments, but got {}",
                            arguments_types.len(),
                            converted_expressions.len()
                        ),
                    })
                } else {
                    let convrted_args = std::iter::zip(arguments_types, converted_expressions)
                        .map(|(arg_type, (expr, expr_type))| cast_to(expr, &expr_type, &arg_type))
                        .collect::<AnalysisResult<Vec<Rc<ast::tree::Expression>>>>()?;
                    Ok((
                        Rc::new(ast::tree::Expression::Call {
                            callee: self.lookup(callee)?.name.clone(),
                            args: convrted_args,
                        }),
                        Rc::clone(return_type),
                    ))
                }
            }
            pt::tree::Expression::Binop { op, lhs, rhs } => {
                let (converted_lhs, lhs_type) = self.convert_expr(lhs)?;
                let (converted_rhs, rhs_type) = self.convert_expr(rhs)?;
                let (result_type, operand_type, semantic_op) =
                    infer_binary_operator_type(&lhs_type, &rhs_type, *op)?;

                let actual_lhs = cast_to(converted_lhs, &lhs_type, &operand_type)?;
                let actual_rhs = cast_to(converted_rhs, &rhs_type, &operand_type)?;

                Ok((
                    Rc::new(ast::tree::Expression::Binop {
                        op: semantic_op,
                        lhs: actual_lhs,
                        rhs: actual_rhs,
                    }),
                    result_type,
                ))
            }
            pt::tree::Expression::Unop { op, operand } => {
                let (converted_operand, operand_type) = self.convert_expr(operand)?;

                match op {
                    SyntacticOperator::Neg => match &*operand_type {
                        ast::types::Type::Bool => Ok((
                            Rc::new(ast::tree::Expression::Unop {
                                op: SemanticUnaryOperator::BoolNeg,
                                operand: converted_operand,
                            }),
                            Rc::new(ast::types::Type::Bool),
                        )),
                        ast::types::Type::Int
                        | ast::types::Type::Real
                        | ast::types::Type::Alias(_)
                        | ast::types::Type::Record(_)
                        | ast::types::Type::Array(_)
                        | ast::types::Type::Null
                        | ast::types::Type::Unit => Err(AnalysisError {
                            what: format!(
                                "Logical negation operator can not be applied to non-boolean {operand_type:?} value"
                            ),
                        }),
                    },
                    SyntacticOperator::Sub => match &*operand_type {
                        ast::types::Type::Int => Ok((
                            Rc::new(ast::tree::Expression::Unop {
                                op: SemanticUnaryOperator::IntNeg,
                                operand: converted_operand,
                            }),
                            Rc::new(ast::types::Type::Int),
                        )),
                        ast::types::Type::Real => Ok((
                            Rc::new(ast::tree::Expression::Unop {
                                op: SemanticUnaryOperator::RealNeg,
                                operand: converted_operand,
                            }),
                            Rc::new(ast::types::Type::Real),
                        )),
                        ast::types::Type::Bool
                        | ast::types::Type::Alias(_)
                        | ast::types::Type::Record(_)
                        | ast::types::Type::Array(_)
                        | ast::types::Type::Null
                        | ast::types::Type::Unit => Err(AnalysisError {
                            what: format!(
                                "Arithmetical negation operator can not be applied to non-scalar type {operand_type:?} value"
                            ),
                        }),
                    },
                    SyntacticOperator::Add
                    | SyntacticOperator::Mul
                    | SyntacticOperator::Div
                    | SyntacticOperator::Mod
                    | SyntacticOperator::Eq
                    | SyntacticOperator::Neq
                    | SyntacticOperator::Lt
                    | SyntacticOperator::Le
                    | SyntacticOperator::Gt
                    | SyntacticOperator::Ge
                    | SyntacticOperator::And
                    | SyntacticOperator::Or
                    | SyntacticOperator::Xor => Err(AnalysisError {
                        what: format!("Operator {op:?} can not be applied as unary"),
                    }),
                }
            }
            pt::tree::Expression::Cast { operand, target } => {
                let (converted_operand, operand_type) = self.convert_expr(operand)?;
                let converted_target_type = self.convert_type(target)?;
                let operand_effective_type = self.ident_table.get_effective_type(&operand_type)?;
                let target_effective_type = self
                    .ident_table
                    .get_effective_type(&converted_target_type)?;

                if operand_effective_type == target_effective_type {
                    Ok((
                        Rc::new(ast::tree::Expression::Cast {
                            operand: converted_operand,
                            target: Rc::clone(&converted_target_type),
                        }),
                        Rc::clone(&converted_target_type),
                    ))
                } else {
                    Err(AnalysisError {
                        what: format!("Type {operand_type:?} is incompatible with {target:?}"),
                    })
                }
            }
            pt::tree::Expression::New { t, fields } => {
                let converted_type = self.convert_type(t)?;
                let converted_effective_type =
                    self.ident_table.get_effective_type(&converted_type)?;

                match &*converted_effective_type {
                    ast::types::Type::Int
                    | ast::types::Type::Real
                    | ast::types::Type::Bool
                    | ast::types::Type::Null
                    | ast::types::Type::Unit => Err(AnalysisError {
                        what: format!("No new operator supprted for built-in type {t:?}"),
                    }),
                    ast::types::Type::Alias(_) => unreachable!("Effective type can not be alias"),
                    ast::types::Type::Record(_) => {
                        let defined_fields = fields.clone().unwrap_or(Vec::new());
                        let converted_fields = defined_fields.iter().map(|(name, expr)| {
                            self.convert_expr(expr).and_then(
                                |(converted_expr, expr_type)| {
                                    Ok((
                                        name.clone(),
                                        cast_to(
                                            converted_expr,
                                            &expr_type,
                                            &(converted_effective_type.get_field_type(name)?),
                                        )?,
                                    ))
                                },
                            )
                        }).collect::<AnalysisResult<Vec<(RawIdentifier, Rc<ast::tree::Expression>)>>>()?;

                        Ok((
                            Rc::new(ast::tree::Expression::New {
                                t: Rc::clone(&converted_type),
                                fields: Some(converted_fields),
                            }),
                            Rc::clone(&converted_type),
                        ))
                    }

                    ast::types::Type::Array(array_description) => {
                        if fields.is_none() && array_description.length.is_some() {
                            Ok((
                                Rc::new(ast::tree::Expression::New {
                                    t: Rc::clone(&converted_type),
                                    fields: None,
                                }),
                                Rc::clone(&converted_type),
                            ))
                        } else if !fields.is_none() {
                            Err(AnalysisError {
                                what: format!(
                                    "No field initialisation possible for array type {t:?}"
                                ),
                            })
                        } else {
                            Err(AnalysisError {
                                what: format!("No new length known array creation {t:?}"),
                            })
                        }
                    }
                }
            }
            pt::tree::Expression::Null => Ok((
                Rc::new(ast::tree::Expression::Null),
                Rc::new(ast::types::Type::Null),
            )),
        }
    }

    pub fn convert_stmt(
        &mut self,
        stmt: &pt::tree::Statement,
    ) -> AnalysisResult<ast::tree::Statement> {
        match stmt {
            pt::tree::Statement::Assignment { lhs, rhs } => {
                let (lhs, target_type) = self.convert_lvalue_expr(lhs)?;
                let (rhs, own_type) = self.convert_expr(rhs)?;
                let converted_rhs = cast_to(rhs, &own_type, &target_type)?;

                Ok(ast::tree::Statement::Assignment {
                    lhs,
                    rhs: converted_rhs,
                })
            }
            pt::tree::Statement::While { condition, body } => {
                let (condition_expr, condition_type) = self.convert_expr(condition)?;
                let converted_condition_expr = cast_to(
                    condition_expr,
                    &condition_type,
                    &Rc::new(ast::types::Type::Bool),
                )?;
                let body = self.convert_block(body)?;
                Ok(ast::tree::Statement::While {
                    condition: converted_condition_expr,
                    body,
                })
            }
            pt::tree::Statement::Expr(expression) => {
                let (expr, _) = self.convert_expr(expression)?;
                Ok(ast::tree::Statement::Expr(expr))
            }
            pt::tree::Statement::If {
                condition,
                on_true,
                on_false,
            } => {
                let (condition_expr, condition_type) = self.convert_expr(condition)?;
                let converted_condition_expr = cast_to(
                    condition_expr,
                    &condition_type,
                    &Rc::new(ast::types::Type::Bool),
                )?;

                let converted_then = self.convert_block(on_true)?;
                let converted_else = on_false
                    .as_ref()
                    .map(|block| self.convert_block(block))
                    .transpose()?;

                Ok(ast::tree::Statement::If {
                    condition: converted_condition_expr,
                    on_true: converted_then,
                    on_false: converted_else,
                })
            }
            pt::tree::Statement::For {
                counter,
                from,
                to,
                order,
                body,
            } => {
                self.enter_block(); // For counter is in it's own block

                let counter_loc = self.get_fresh_local_location();

                let stmt = match to {
                    None => {
                        let (array_expr, array_type) = self.convert_expr(from)?;
                        let element_type = array_type.get_element_type()?;
                        let counter_decl = ast::tree::Decl::Var(ast::tree::VarDecl {
                            t: Rc::clone(&element_type),
                            initialiser: None,
                            relative_location: counter_loc,
                        });

                        let counter_ident = self.bind_local_decl(counter, counter_decl);
                        let body = self.convert_block(body)?;
                        ast::tree::Statement::ForEach {
                            counter: counter_ident,
                            collection: array_expr,
                            order: *order,
                            body,
                        }
                    }
                    Some(to) => {
                        let (lower_bound, lower_bound_type) = self.convert_expr(from)?;
                        let (upper_bound, upper_bound_type) = self.convert_expr(to)?;
                        let int_type = Rc::new(ast::types::Type::Int);
                        let counter_decl = ast::tree::Decl::Var(ast::tree::VarDecl {
                            t: Rc::clone(&int_type),
                            initialiser: None,
                            relative_location: counter_loc,
                        });
                        let counter_ident = self.bind_local_decl(counter, counter_decl);
                        let body = self.convert_block(body)?;
                        ast::tree::Statement::For {
                            counter: counter_ident,
                            lower_bound: cast_to(lower_bound, &lower_bound_type, &int_type)?,
                            upper_bound: cast_to(upper_bound, &upper_bound_type, &int_type)?,
                            order: *order,
                            body,
                        }
                    }
                };

                self.leave_block();
                Ok(stmt)
            }
            pt::tree::Statement::Print { value } => {
                let (expr, _) = self.convert_expr(value)?;
                Ok(ast::tree::Statement::Print { value: expr })
            }
            pt::tree::Statement::Return { value } => {
                let (expr, own_type) = self.convert_expr(value)?;
                match &self.current_routine {
                    Some(RoutinePrototype { return_type, .. }) => {
                        let converted_expr = cast_to(expr, &own_type, return_type)?;
                        Ok(ast::tree::Statement::Return {
                            value: converted_expr,
                        })
                    }
                    None => Err(AnalysisError {
                        what: "Return outside of routine".to_string(),
                    }),
                }
            }
        }
    }

    pub fn convert_block(&mut self, block: &pt::tree::Block) -> AnalysisResult<ast::tree::Block> {
        let mut result: Vec<ast::tree::BlockElem> = Vec::new();

        self.enter_block();

        for block_elem in &block.0 {
            result.push(match block_elem {
                pt::tree::BlockElem::Stmt(statement) => {
                    ast::tree::BlockElem::Stmt(self.convert_stmt(statement)?)
                }
                pt::tree::BlockElem::VarDecl(var_decl) => ast::tree::BlockElem::Decl(
                    self.convert_decl(&pt::tree::Declaration::Var(var_decl.clone()), false)
                        .and_then(ast::tree::Binding::to_simple_binding)?,
                ),
                pt::tree::BlockElem::TypeDecl(type_decl) => ast::tree::BlockElem::Decl(
                    self.convert_decl(&pt::tree::Declaration::Type(type_decl.clone()), false)
                        .and_then(ast::tree::Binding::to_simple_binding)?,
                ),
            })
        }

        self.leave_block();

        Ok(ast::tree::Block(result))
    }

    pub fn convert_decl(
        &mut self,
        decl: &pt::tree::Declaration,
        is_global: bool,
    ) -> AnalysisResult<ast::tree::Binding> {
        match decl {
            pt::tree::Declaration::Var(pt::tree::VarDecl {
                name,
                t,
                initialiser,
            }) => {
                let loc = if is_global {
                    self.get_fresh_global_location()
                } else {
                    self.get_fresh_local_location()
                };

                let var_decl: ast::tree::Decl = match (t, initialiser) {
                    (None, None) => Err(AnalysisError {
                        what: format!("Can not deduce type for variable {name:?}"),
                    }),
                    (None, Some(expr)) => {
                        let (converted_initialiser, t) = self.convert_expr(expr)?;
                        Ok(ast::tree::Decl::Var(ast::tree::VarDecl {
                            t,
                            initialiser: Some(converted_initialiser),
                            relative_location: loc,
                        }))
                    }
                    (Some(t), None) => {
                        let converted_type = self.convert_type(t)?;
                        Ok(ast::tree::Decl::Var(ast::tree::VarDecl {
                            t: converted_type,
                            initialiser: None,
                            relative_location: loc,
                        }))
                    }
                    (Some(t), Some(expr)) => {
                        let converted_type = self.convert_type(t)?;
                        let (converted_initialiser, init_type) = self.convert_expr(expr)?;
                        Ok(ast::tree::Decl::Var(ast::tree::VarDecl {
                            t: Rc::clone(&converted_type),
                            initialiser: Some(cast_to(
                                converted_initialiser,
                                &init_type,
                                &converted_type,
                            )?),
                            relative_location: loc,
                        }))
                    }
                }?;

                let ident = self.bind_decl(is_global, name, var_decl.clone());
                Ok(ast::tree::Binding {
                    name: ident,
                    decl: var_decl,
                })
            }
            pt::tree::Declaration::Type(pt::tree::TypeDecl { name, t }) => {
                // Binding forward declaration of type for possible recursive usage
                let ident = self.bind_decl(
                    is_global,
                    name,
                    ast::tree::Decl::Type(ast::tree::TypeDecl::Forward {
                        alias: name.clone(),
                    }),
                );

                let converted_type = self.convert_type(t)?;
                let effective_type = self.ident_table.get_effective_type(&converted_type)?;
                let type_decl = ast::tree::Decl::Type(ast::tree::TypeDecl::Full {
                    prescribed: converted_type,
                    effective: effective_type,
                });
                // Overriding forward declaration with full one
                self.rebind_decl(&ident, type_decl.clone());
                Ok(ast::tree::Binding {
                    name: ident,
                    decl: type_decl,
                })
            }
            pt::tree::Declaration::Routine(pt::tree::RoutineDecl {
                name,
                arguments,
                return_type,
                body,
            }) => {
                if !is_global {
                    return Err(AnalysisError {
                        what: "Local routines declarations is not supported".to_string(),
                    });
                }
                let converted_arguments_types = arguments
                    .iter()
                    .map(|(name, arg_type)| {
                        self.convert_type(arg_type)
                            .map(|conerted_arg_type| (name.clone(), conerted_arg_type))
                    })
                    .collect::<AnalysisResult<Vec<(RawIdentifier, Rc<ast::types::Type>)>>>()?;

                match body {
                    None => {
                        let return_type = match &return_type {
                            Some(t) => self.convert_type(t)?,
                            None => Rc::new(ast::types::Type::Unit),
                        };

                        let signature = ast::tree::RoutineSignature {
                            args: converted_arguments_types,
                            return_type,
                        };

                        let ident = self.bind_routine(
                            name,
                            ast::tree::RoutineDecl::Forward {
                                signature: signature.clone(),
                            },
                        )?;

                        Ok(ast::tree::Binding {
                            name: ident,
                            decl: ast::tree::Decl::Routine(ast::tree::RoutineDecl::Forward {
                                signature,
                            }),
                        })
                    }
                    Some(body) => {
                        let args_decls = converted_arguments_types
                            .iter()
                            .enumerate()
                            .map(|(index, (name, t))| {
                                (
                                    name.clone(),
                                    ast::tree::Decl::Var(ast::tree::VarDecl {
                                        t: Rc::clone(t),
                                        initialiser: None,
                                        relative_location: Location::Argument(
                                            u16::try_from(index)
                                                .expect("Too many arguments for function"),
                                        ),
                                    }),
                                )
                            })
                            .collect::<Vec<(RawIdentifier, ast::tree::Decl)>>();

                        match (return_type, body) {
                            (return_type, pt::tree::RoutineBody::Block(block)) => {
                                let converted_return_type = return_type
                                    .as_ref()
                                    .map_or(Ok(Rc::new(ast::types::Type::Unit)), |t| {
                                        self.convert_type(t)
                                    })?;

                                // Firstly, create a forward declaration for possible recursive use
                                let signature = ast::tree::RoutineSignature {
                                    args: converted_arguments_types.clone(),
                                    return_type: Rc::clone(&converted_return_type),
                                };

                                let ident = self.bind_routine(
                                    name,
                                    ast::tree::RoutineDecl::Forward {
                                        signature: signature.clone(),
                                    },
                                )?;

                                // Memorise that routine for type-check of return statement
                                self.current_routine = Some(RoutinePrototype {
                                    name: name.clone(),
                                    args: converted_arguments_types
                                        .iter()
                                        .map(|(_, t)| t)
                                        .cloned()
                                        .collect::<Vec<Rc<ast::types::Type>>>()
                                        .clone(),
                                    return_type: Rc::clone(&converted_return_type),
                                });

                                // create a function scope
                                self.enter_block();

                                let args_bindings = args_decls
                                    .iter()
                                    .map(|(raw_name, decl)| {
                                        let arg_ident =
                                            self.bind_local_decl(raw_name, decl.clone());
                                        ast::tree::Binding {
                                            name: arg_ident,
                                            decl: decl.clone(),
                                        }
                                    })
                                    .collect::<Vec<ast::tree::Binding>>();

                                let converted_body = self.convert_block(block)?;

                                self.leave_block();
                                self.current_routine = None;

                                Ok(ast::tree::Binding {
                                    name: ident,
                                    decl: ast::tree::Decl::Routine(ast::tree::RoutineDecl::Full {
                                        signature,
                                        args_bindings,
                                        body: ast::tree::RoutineBody::Block(converted_body),
                                    }),
                                })
                            }
                            (None, pt::tree::RoutineBody::Expression(expression)) => {
                                // No recursive calls for expression function
                                self.enter_block();

                                let args_bindings = args_decls
                                    .iter()
                                    .map(|(raw_name, decl)| {
                                        let arg_ident =
                                            self.bind_local_decl(raw_name, decl.clone());
                                        ast::tree::Binding {
                                            name: arg_ident,
                                            decl: decl.clone(),
                                        }
                                    })
                                    .collect::<Vec<ast::tree::Binding>>();

                                let (expr, t) = self.convert_expr(expression)?;
                                self.leave_block(); // If we met an error we do not need recover

                                let signature = ast::tree::RoutineSignature {
                                    args: converted_arguments_types,
                                    return_type: t,
                                };

                                let decl = ast::tree::RoutineDecl::Full {
                                    signature,
                                    args_bindings,
                                    body: ast::tree::RoutineBody::Expression(expr),
                                };

                                let ident = self.bind_routine(name, decl.clone())?;
                                Ok(ast::tree::Binding {
                                    name: ident,
                                    decl: ast::tree::Decl::Routine(decl),
                                })
                            }
                            (Some(return_type), pt::tree::RoutineBody::Expression(expression)) => {
                                let converted_return_type = self.convert_type(return_type)?;

                                let args_bindings = args_decls
                                    .iter()
                                    .map(|(raw_name, decl)| {
                                        let arg_ident =
                                            self.bind_local_decl(raw_name, decl.clone());
                                        ast::tree::Binding {
                                            name: arg_ident,
                                            decl: decl.clone(),
                                        }
                                    })
                                    .collect::<Vec<ast::tree::Binding>>();

                                let (expr, t) = self.convert_expr(expression)?;
                                self.leave_block(); // If we met an error we do not need recover

                                let expr = cast_to(expr, &t, &converted_return_type)?;

                                let signature = ast::tree::RoutineSignature {
                                    args: converted_arguments_types,
                                    return_type: converted_return_type,
                                };

                                let decl = ast::tree::RoutineDecl::Full {
                                    signature,
                                    args_bindings,
                                    body: ast::tree::RoutineBody::Expression(expr),
                                };

                                let ident = self.bind_routine(name, decl.clone())?;
                                Ok(ast::tree::Binding {
                                    name: ident,
                                    decl: ast::tree::Decl::Routine(decl),
                                })
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn convert(
    program: &pt::tree::Program,
) -> AnalysisResult<(ast::tree::Program, IdentifierTable)> {
    let mut converter = Converter::new();

    let converted_program = program
        .0
        .iter()
        .map(|decl| converter.convert_decl(decl, true))
        .collect::<AnalysisResult<Vec<ast::tree::Binding>>>()
        .map(ast::tree::Program)?;

    Ok((converted_program, converter.extract_table()))
}
