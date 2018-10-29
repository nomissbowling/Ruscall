use combine::stream::state::SourcePosition;
use std::collections::HashMap;
use super::super::ast::*;

#[derive(Debug, PartialEq)]
pub struct ProgramIr {
    pub dec_func_list: Vec<DecFuncIr>,

    pub func_list: Vec<FuncIr>,
    //extern　宣言された関数のリスト
    pub ex_dec_func_list: Vec<DecFuncIr>,
}

impl ProgramIr {
    pub fn empty() -> ProgramIr {
        ProgramIr {
            dec_func_list: vec![],
            func_list: vec![],
            ex_dec_func_list: vec![],
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FuncIr {
    pub name: String,
    pub body: ExprIr,
    pub params_len: usize,
    pub pos: SourcePosition,
}

#[derive(Debug, PartialEq)]
pub enum ExprIr {
    OpIr(Box<OpIr>),
    NumIr(NumIr),
    TupleIr(Box<TupleIr>),
    VariableIr(VariableIr),
    GlobalVariableIr(GlobalVariableIr),
    CallIr(Box<CallIr>),
    LambdaIr(Box<LambdaIr>),
}

impl ExprIr {
    pub fn create_opir(op: String, l_expr: ExprIr, r_expr: ExprIr) -> ExprIr {
        ExprIr::OpIr(Box::new(OpIr {
            op,
            l_expr,
            r_expr,
        }))
    }
    pub fn create_variableir(id: usize, pos: SourcePosition) -> ExprIr {
        ExprIr::VariableIr(VariableIr {
            id,
            pos,
        })
    }
    pub fn create_global_variableir(id: String, pos: SourcePosition) -> ExprIr {
        ExprIr::GlobalVariableIr(GlobalVariableIr {
            id,
            pos,
        })
    }
    pub fn create_callir(func: ExprIr, params: Vec<ExprIr>) -> ExprIr {
        ExprIr::CallIr(Box::new(CallIr {
            func,
            params,
        }))
    }

    pub fn create_tupleir(elements: Vec<ExprIr>, pos: SourcePosition) -> ExprIr {
        ExprIr::TupleIr(Box::new(TupleIr {
            elements,
            pos,
        }))
    }

    pub fn create_lambdair(env: Vec<VariableIr>, params_len: usize, body: ExprIr, pos: SourcePosition, id: String) -> ExprIr {
        ExprIr::LambdaIr(Box::new(LambdaIr {
            env,
            func: FuncIr { name: id, params_len, body, pos },
        }))
    }
}

#[derive(Debug, PartialEq)]
pub struct OpIr {
    pub op: String,
    pub l_expr: ExprIr,
    pub r_expr: ExprIr,
}

pub type NumIr = NumAST;

#[derive(Debug, PartialEq)]
pub struct TupleIr {
    pub elements: Vec<ExprIr>,
    pub pos: SourcePosition,
}

#[derive(Debug, PartialEq)]
pub struct VariableIr {
    pub id: usize,
    pub pos: SourcePosition,
}

pub type GlobalVariableIr = VariableAST;

#[derive(Debug, PartialEq)]
pub struct CallIr {
    pub func: ExprIr,
    pub params: Vec<ExprIr>,
}

#[derive(Debug, PartialEq)]
pub struct LambdaIr {
    pub env: Vec<VariableIr>,
    pub func: FuncIr,
}

pub type DecFuncIr = DecFuncAST;