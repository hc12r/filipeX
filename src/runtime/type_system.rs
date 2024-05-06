use super::{object::Object, Runtime, Expr, ExprType, Identifier, Literal};

#[derive(PartialEq, Clone, Debug)]
pub enum Type {
    Null,
    Void,
    Int,
    Float,
    String,
    Boolean,
    Function,
    Range,
    Array,
    TypeAnnot,
    Unknown,
}

pub fn expr_type_to_object_type(var_type: &ExprType) -> Type {
    match var_type {
        ExprType::String => Type::String,
        ExprType::Boolean => Type::Boolean,
        ExprType::Void => Type::Void,
        ExprType::Int => Type::Int,
        ExprType::Float => Type::Float,
        ExprType::Array => Type::Array,
    }
}

pub fn expr_to_type(e: &mut Runtime, expr: &Expr) -> Option<Type> {
    match expr {
        Expr::Literal(literal) => match literal {
            Literal::String(_) => return Some(Type::String),
            Literal::Null => return Some(Type::Null),
            Literal::Boolean(_) => return Some(Type::Boolean),
            Literal::Int(_) => return Some(Type::Int),
            Literal::Float(_) => return Some(Type::Float),
            Literal::Array(_) => return Some(Type::Array),
        },
        Expr::Identifier(identifier) => return identifier_to_type(e, identifier),
        _ => {
            return None;
        }
    }
}

pub fn has_same_type(lhs: &Object, rhs: &Object) -> bool {
    object_to_type(lhs) == object_to_type(rhs)
}

pub fn identifier_to_type(e: &mut Runtime, identifier: &Identifier) -> Option<Type> {
    let Identifier(name) = identifier;

    match e.env.borrow().get_typeof(&name) {
        Some(type_) => Some(type_),
        None => {
            e.error_handler
                .set_name_error(format!("'{}' is not declared", &name));
            return None;
        }
    }
}

pub fn object_to_type(object: &Object) -> Type {
    match object {
        Object::Null => Type::Null,
        Object::String(_) => Type::String,
        Object::Boolean(_) => Type::Boolean,
        Object::BuiltInFunction(_) => Type::Function,
        Object::UserDefinedFunction {
            name: _,
            params: _,
            body: _,
            return_type: _,
        } => Type::Function,
        Object::RetVal(val) => object_to_type(&val),
        Object::Type(_) => Type::TypeAnnot,
        Object::Range { start: _, end: _ } => Type::Range,
        Object::Int(_) => Type::Int,
        Object::Float(_) => Type::Float,
        Object::Array(_, _) => Type::Array,
    }
}