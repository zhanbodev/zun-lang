use crate::ast::TypeDeclStmt;
use crate::runtime::{
    RuntimeContext,
    context::{TypeField, TypeShape},
};

use super::Flow;

pub(super) fn handle_type_decl(stmt: &TypeDeclStmt, context: &mut RuntimeContext) -> Flow {
    let shape = TypeShape {
        name: stmt.name.clone(),
        fields: stmt
            .fields
            .iter()
            .map(|f| TypeField {
                name: f.name.clone(),
                type_expr: f.type_expr.clone(),
                hidden: f.hidden,
            })
            .collect(),
    };
    context.register_type(stmt.name.clone(), shape);
    Flow::Normal
}
