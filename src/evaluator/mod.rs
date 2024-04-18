pub mod environment;
mod evaluators;
pub mod flstdlib;
pub mod object;
mod runtime_error;
mod type_system;

use crate::ast::*;
use environment::Environment;
use evaluators::func_call_evaluator::eval_call_expr;
use evaluators::func_def_evaluator::eval_func_def;
use evaluators::let_evaluator::eval_let_stmt;
use object::Object;
use runtime_error::RuntimeErrorHandler;
use type_system::{has_same_type, object_to_type, Type};

pub struct Evaluator<'a> {
    env: &'a mut Environment,
    pub error_handler: RuntimeErrorHandler,
}

impl<'a> Evaluator<'a> {
    pub fn new(env: &'a mut Environment) -> Self {
        Self {
            env,
            error_handler: RuntimeErrorHandler::new(),
        }
    }

    pub fn eval(&mut self, program: Program) -> Option<Object> {
        let mut output: Option<Object> = None;
        for stmt in program {
            let object = self.eval_stmt(&stmt);
            if self.error_handler.has_error() {
                eprintln!("{}", self.error_handler.get_error().unwrap());
                return None;
            }
            output = object;
        }
        output
    }

    fn eval_stmt(&mut self, stmt: &Stmt) -> Option<Object> {
        match stmt {
            Stmt::Let(name, var_type, expr) => {
                eval_let_stmt(self, name, var_type, expr);
                None
            }
            Stmt::Func(identifier, params, body, ret_type) => {
                eval_func_def(self, identifier, params, body, ret_type);
                None
            }
            Stmt::Return(expr) => self.eval_return(expr),
            Stmt::Expr(expr) => self.eval_expr(expr),
            Stmt::If {
                condition,
                consequence,
                alternative,
            } => self.eval_if_stmt(condition, consequence, alternative),
            Stmt::ForLoop {
                cursor,
                iterable,
                block,
            } => self.eval_forloop_stmt(cursor, iterable, block),
        }
    }

    fn eval_forloop_stmt(
        &mut self,
        cursor: &String,
        iterable: &Expr,
        block: &BlockStmt,
    ) -> Option<Object> {
        let iterable_object = match self.eval_expr(iterable) {
            Some(object) => object,
            None => return None,
        };
        match iterable_object {
            Object::Range { start, end } => self.eval_range_forloop(cursor, start, end, block),
            _ => {
                self.error_handler
                    .set_type_error(format!("for loop works only with range (for now)"));
                return None;
            }
        }
    }

    fn eval_range_forloop(
        &mut self,
        cursor: &String,
        start: i64,
        end: i64,
        block: &BlockStmt,
    ) -> Option<Object> {
        let global_scope = self.env.clone();
        let block_scope = Environment::empty(Some(self.env.clone()));
        *self.env = block_scope;

        self.env.add_entry(
            cursor.clone(),
            Object::Int(start),
            Type::Int,
            true,
        );

        for _ in start..end {
            self.eval_block_stmt(block);
            let old_val = match self.env.resolve(&cursor).unwrap().value {
                Object::Int(val) => val,
                _ => return None,
            };
            self.env
                .update_entry(&cursor, Object::Int(old_val + 1));
        }

        *self.env = global_scope;
        None
    }

    fn is_truthy(&mut self, object: Object) -> bool {
        match object {
            Object::Null | Object::Boolean(false) => false,
            Object::Int(val) => val != 0,
            Object::Float(val) => val != 0.0,
            _ => true,
        }
    }

    fn eval_if_stmt(
        &mut self,
        condition: &Expr,
        consequence: &BlockStmt,
        alternative: &Option<BlockStmt>,
    ) -> Option<Object> {
        let evaluated_cond = match self.eval_expr(condition) {
            Some(object) => object,
            None => return None,
        };

        if self.is_truthy(evaluated_cond) {
            return self.eval_block_stmt(consequence);
        }

        if alternative.is_some() {
            return self.eval_block_stmt(&alternative.clone().unwrap());
        }

        None
    }

    fn eval_expr(&mut self, expr: &Expr) -> Option<Object> {
        match expr {
            Expr::Literal(literal) => Some(self.eval_literal_expr(literal)),
            Expr::Identifier(identifier) => self.resolve_identfier(identifier),
            Expr::Call(func, args) => eval_call_expr(self, func, args),
            Expr::Infix(lhs, infix, rhs) => self.eval_infix_expr(lhs, infix, rhs),
            Expr::Prefix(prefix, expr) => self.eval_prefix_expr(prefix, expr),
            Expr::Postfix(expr, postfix) => self.eval_postfix_expr(expr, postfix),
            Expr::Assign(identifier, expr) => self.eval_assign_expr(identifier, expr),
        }
    }

    fn eval_postfix_expr(&mut self, expr: &Expr, postfix: &Postfix) -> Option<Object> {
        let evaluated_expr = match self.eval_expr(expr) {
            Some(object) => object,
            None => return None,
        };

        let old_value = match evaluated_expr {
            Object::Int(val) => val,
            _ => {
                self.error_handler.set_type_error(format!(
                    "'{}' operation is only allowed for type 'number'",
                    postfix
                ));
                return None;
            }
        };

        match postfix {
            Postfix::Increment => Some(Object::Int(old_value + 1)),
            Postfix::Decrement => Some(Object::Int(old_value - 1)),
        }
    }

    fn eval_prefix_expr(&mut self, prefix: &Prefix, expr: &Expr) -> Option<Object> {
        let evaluated_expr = match self.eval_expr(expr) {
            Some(object) => object,
            None => return None,
        };

        match prefix {
            Prefix::Not => self.eval_not_prefix(&evaluated_expr),
            Prefix::Plus => self.eval_plus_prefix(prefix, &evaluated_expr),
            Prefix::Minus => self.eval_minus_prefix(prefix, &evaluated_expr),
        }
    }

    fn eval_not_prefix(&mut self, evaluated_expr: &Object) -> Option<Object> {
        match evaluated_expr {
            Object::Null => Some(Object::Boolean(true)),
            Object::Boolean(val) => Some(Object::Boolean(!val)),
            _ => Some(Object::Boolean(false)),
        }
    }

    fn eval_plus_prefix(&mut self, prefix: &Prefix, evaluated_expr: &Object) -> Option<Object> {
        match evaluated_expr {
            Object::Int(val) => Some(Object::Int(val.clone())),
            Object::Float(val) => Some(Object::Float(val.clone())),
            _ => {
                self.error_handler
                    .set_type_error(format!("'{}' prefix is for type number", prefix));
                return None;
            }
        }
    }

    fn eval_minus_prefix(&mut self, prefix: &Prefix, evaluated_expr: &Object) -> Option<Object> {
        match evaluated_expr {
            Object::Int(val) => Some(Object::Int(-val)),
            Object::Float(val) => Some(Object::Float(-val)),
            _ => {
                self.error_handler
                    .set_type_error(format!("'{}' prefix is for type number", prefix));
                return None;
            }
        }
    }

    fn eval_return(&mut self, expr: &Option<Expr>) -> Option<Object> {
        if expr.is_none() {
            return Some(Object::RetVal(Box::new(Object::Null)));
        }
        match self.eval_expr(&expr.clone().unwrap()) {
            Some(object) => Some(Object::RetVal(Box::new(object))),
            None => None,
        }
    }

    fn eval_assign_expr(&mut self, identifier: &Identifier, expr: &Expr) -> Option<Object> {
        let Identifier(name) = identifier;
        if !self.env.is_declared(&name) {
            self.error_handler
                .set_name_error(format!("'{}' is not declared", &name));
            return None;
        }
        let value = match self.eval_expr(expr) {
            Some(value) => value,
            None => return None,
        };

        let old_value_type = self.env.get_typeof(&name).unwrap();
        let new_value_type = object_to_type(&value);

        if old_value_type != new_value_type {
            self.error_handler.set_type_error(format!(
                "can't assign value of type '{}' to value of type '{}'",
                old_value_type, new_value_type
            ));
            return None;
        }

        if !self.env.update_entry(&name, value) {
            self.error_handler
                .set_name_error(format!("'{}' is not assignable", &name));
        }
        None
    }

    fn eval_block_stmt(&mut self, block: &BlockStmt) -> Option<Object> {
        let mut res = None;

        for stmt in block {
            match self.eval_stmt(stmt) {
                Some(Object::RetVal(object)) => return Some(*object),
                object => res = object,
            }
        }

        res
    }

    fn eval_infix_expr(&mut self, lhs: &Expr, infix: &Infix, rhs: &Expr) -> Option<Object> {
        let lhs = self.eval_expr(lhs);
        let rhs = self.eval_expr(rhs);

        if lhs.is_none() || rhs.is_none() {
            return None;
        }

        let lhs = lhs.unwrap();
        let rhs = rhs.unwrap();

        if !has_same_type(&lhs, &rhs) {
            self.error_handler.set_type_error(format!(
                "'{}' operation not allowed between types {} and {}",
                infix,
                object_to_type(&lhs),
                object_to_type(&rhs),
            ));
            return None;
        }

        match lhs {
            Object::Int(lval) => {
                if let Object::Int(rval) = rhs {
                    return Some(self.eval_infix_int_expr(lval, infix, rval));
                }
                None
            }
            Object::Float(lval) => {
                if let Object::Float(rval) = rhs {
                    return Some(self.eval_infix_float_expr(lval, infix, rval));
                }
                None
            }
            Object::String(lval) => {
                if let Object::String(rval) = rhs {
                    return Some(self.eval_infix_string_expr(&lval, infix, &rval));
                }
                None
            }
            Object::Boolean(lval) => {
                if let Object::Boolean(rval) = rhs {
                    return Some(self.eval_infix_bool_expr(&lval, infix, &rval));
                }
                None
            }
            _ => None,
        }
    }

    fn eval_infix_string_expr(&mut self, lhs: &String, infix: &Infix, rhs: &String) -> Object {
        match infix {
            Infix::Plus => Object::String(lhs.clone() + rhs),
            Infix::NotEqual => Object::Boolean(lhs != rhs),
            Infix::Equal => Object::Boolean(lhs == rhs),
            _ => {
                self.error_handler.set_type_error(format!(
                    "'{}' operation not implemented for type string",
                    infix
                ));
                Object::Null
            }
        }
    }

    fn eval_infix_int_expr(&mut self, lhs_val: i64, infix: &Infix, rhs_val: i64) -> Object {
        match infix {
            Infix::Plus => Object::Int(lhs_val + rhs_val),
            Infix::Minus => Object::Int(lhs_val - rhs_val),
            Infix::Devide => Object::Int(lhs_val / rhs_val),
            Infix::Multiply => Object::Int(lhs_val * rhs_val),
            Infix::Remainder => Object::Int(lhs_val % rhs_val),
            Infix::Equal => Object::Boolean(lhs_val == rhs_val),
            Infix::LessThan => Object::Boolean(lhs_val < rhs_val),
            Infix::LessOrEqual => Object::Boolean(lhs_val <= rhs_val),
            Infix::GratherThan => Object::Boolean(lhs_val > rhs_val),
            Infix::GratherOrEqual => Object::Boolean(lhs_val >= rhs_val),
            Infix::NotEqual => Object::Boolean(lhs_val != rhs_val),
        }
    }

    fn eval_infix_float_expr(&mut self, lhs_val: f64, infix: &Infix, rhs_val: f64) -> Object {
        match infix {
            Infix::Plus => Object::Float(lhs_val + rhs_val),
            Infix::Minus => Object::Float(lhs_val - rhs_val),
            Infix::Devide => Object::Float(lhs_val / rhs_val),
            Infix::Multiply => Object::Float(lhs_val * rhs_val),
            Infix::Remainder => Object::Float(lhs_val % rhs_val),
            Infix::Equal => Object::Boolean(lhs_val == rhs_val),
            Infix::LessThan => Object::Boolean(lhs_val < rhs_val),
            Infix::LessOrEqual => Object::Boolean(lhs_val <= rhs_val),
            Infix::GratherThan => Object::Boolean(lhs_val > rhs_val),
            Infix::GratherOrEqual => Object::Boolean(lhs_val >= rhs_val),
            Infix::NotEqual => Object::Boolean(lhs_val != rhs_val),
        }
    }

    fn eval_infix_bool_expr(&mut self, lhs_val: &bool, infix: &Infix, rhs_val: &bool) -> Object {
        match infix {
            Infix::Equal => Object::Boolean(lhs_val == rhs_val),
            Infix::LessThan => Object::Boolean(lhs_val < rhs_val),
            Infix::LessOrEqual => Object::Boolean(lhs_val <= rhs_val),
            Infix::GratherThan => Object::Boolean(lhs_val > rhs_val),
            Infix::GratherOrEqual => Object::Boolean(lhs_val >= rhs_val),
            Infix::NotEqual => Object::Boolean(lhs_val != rhs_val),
            _ => {
                self.error_handler.set_type_error(format!(
                    "'{}' operation not implemented for type boolean",
                    infix
                ));
                Object::Null
            }
        }
    }

    fn eval_literal_expr(&mut self, literal: &Literal) -> Object {
        match literal {
            Literal::String(val) => Object::String(val.clone()),
            Literal::Boolean(val) => Object::Boolean(*val),
            Literal::Null => Object::Null,
            Literal::Int(val) => Object::Int(*val),
            Literal::Float(val) => Object::Float(*val),
        }
    }

    fn resolve_identfier(&mut self, identifier: &Identifier) -> Option<Object> {
        let Identifier(name) = identifier;
        let object = match self.env.resolve(&name) {
            Some(object) => object,
            None => {
                self.error_handler
                    .set_name_error(format!("'{}' is not declared", &name));
                return None;
            }
        };
        Some(object.value)
    }

    fn expr_to_identifier(expr: &Expr) -> Option<Identifier> {
        match expr {
            Expr::Identifier(ident) => Some(ident.clone()),
            _ => None,
        }
    }
}
