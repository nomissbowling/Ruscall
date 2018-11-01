use super::super::my_llvm::easy::*;
use super::semantic_analysis::ir as ir;
use super::types::*;
use super::semantic_analysis::type_env::TypeInfo;

pub struct CodeGenResult<'a> {
    pub file_name: &'a str,
    pub code_gen: CodeGenerator,
    pub module: Module,
}

//コード生成する関数
impl ir::ProgramIr {
    pub fn code_gen(self, file_name: &str, mut ty_info: TypeInfo) -> CodeGenResult {
        //llvm初期化
        init_llvm_all_target();
        let code_gen = CodeGenerator::new();
        let module = Module::new(file_name);

        //外部関数宣言のコード化
        self.ex_dec_func_list
            .into_iter()
            .for_each(|x| ex_func_gen(x, &module, &mut ty_info));

        //関数宣言のコード化
        self.func_list.iter().for_each(|x| {
            let func_type =
                ty_info.look_up_name(x.name.clone())
                    .to_llvm_type(false);
            let func = Function::new(&x.name, &module, func_type);
            set_linkage(func.llvm_function, LLVMLinkage::LLVMExternalLinkage);
        });

        //関数定義のコード化
        self.func_list
            .into_iter()
            .for_each(|func| func.code_gen(&module, &code_gen, &mut ty_info));

        module.dump_module();
        if let Some(err_msg) = module.verify_module() {
            panic!("llvm error:{}", err_msg);
        }

        CodeGenResult {
            file_name,
            module,
            code_gen,
        }
    }
}

fn ex_func_gen(dec_func_ir: ir::DecFuncIr, module: &Module, mut ty_info: &mut TypeInfo) {
    match ty_info.look_up_name(dec_func_ir.name.clone()) {
        Type::LambdaType(ty) => {
            let function = Function::new(
                &dec_func_ir.name,
                &module,
                ty.to_llvm_type(false),
            );
            set_linkage(function.llvm_function, LLVMLinkage::LLVMExternalLinkage);
        }
        _ => panic!("error!")
    }
}

impl Type {
    fn to_llvm_type(&self, fn_pointer_flag: bool) -> LLVMTypeRef {
        match self {
            Type::Int32 => int32_type(),
            Type::TupleType(x) => x.to_llvm_type(),
            Type::TyVar(_, _) => panic!("TyVar type!"),
            Type::LambdaType(x) => x.to_llvm_type(fn_pointer_flag)
        }
    }
}

impl FuncType {
    fn to_llvm_type(&self) -> LLVMTypeRef {
        function_type(
            self.ret_type.to_llvm_type(true),
            self.param_types.iter()
                .map(|x| x.to_llvm_type(true))
                .collect(),
        )
    }
}

impl TupleType {
    fn to_llvm_type(&self) -> LLVMTypeRef {
        struct_type(
            self.element_tys.iter().map(|x| x.to_llvm_type(true)).collect(),
        )
    }
}

impl LambdaType {
    fn to_llvm_type(&self, fn_pointer_flag: bool) -> LLVMTypeRef {
        if fn_pointer_flag {
            if let Some(env_ty) = self.env_ty.clone() {
                let env_llvm_ty =
                    env_ty.to_llvm_type();
                let func_llvm_ty = pointer_type(self.func_ty.to_llvm_type());
                struct_type(vec![env_llvm_ty, func_llvm_ty])
            } else {
                pointer_type(self.func_ty.to_llvm_type())
            }
        } else {
            self.func_ty.to_llvm_type()
        }
    }
}

impl ir::ExprIr {
    fn get_ty(&self, ty_info: &mut TypeInfo) -> Type {
        match self {
            ir::ExprIr::NumIr(_) => Type::Int32,
            ir::ExprIr::OpIr(_) => Type::Int32,
            ir::ExprIr::VariableIr(x) => ty_info.look_up(&x.ty_id,&vec![]),
            ir::ExprIr::GlobalVariableIr(x) => ty_info.look_up_name(x.id.clone()),
            ir::ExprIr::CallIr(x) => ty_info.look_up(&x.ty_id,&vec![]),
            ir::ExprIr::TupleIr(x) => ty_info.look_up(&x.ty_id,&vec![]),
            ir::ExprIr::LambdaIr(x) => ty_info.look_up(&x.ty_id,&vec![])
        }
    }
}

impl ir::FuncIr {
    fn code_gen(self, module: &Module, codegen: &CodeGenerator, ty_info: &mut TypeInfo) {
        let function = module.get_named_function(&self.name);
        let params = function.get_params(self.params_len);
        let entry_block = function.append_basic_block(&"entry");
        codegen.position_builder_at_end(entry_block);
        let value = self.body.code_gen(&module, &codegen, &params, ty_info);
        codegen.position_builder_at_end(entry_block);
        codegen.build_ret(value);
    }
}

impl ir::ExprIr {
    fn code_gen(
        self,
        module: &Module,
        codegen: &CodeGenerator,
        params: &Vec<LLVMValueRef>,
        ty_info: &mut TypeInfo,
    ) -> LLVMValueRef {
        match self {
            ir::ExprIr::NumIr(num_ir) => const_int(int32_type(), num_ir.num as u64, true),
            ir::ExprIr::OpIr(op_ir) => op_ir.code_gen(module, codegen, params, ty_info),
            ir::ExprIr::VariableIr(var_ir) => params[params.len() - var_ir.id - 1],
            ir::ExprIr::GlobalVariableIr(x) => x.code_gen(module),
            ir::ExprIr::CallIr(x) => x.code_gen(module, codegen, params, ty_info),
            ir::ExprIr::TupleIr(x) => x.code_gen(module, codegen, params, ty_info),
            ir::ExprIr::LambdaIr(x) => x.code_gen(module, codegen, params, ty_info)
        }
    }
}

impl ir::GlobalVariableIr {
    fn code_gen(self, module: &Module) -> LLVMValueRef {
        let func = module.get_named_function(&self.id);
        func.llvm_function
    }
}

impl ir::CallIr {
    fn code_gen(
        self,
        module: &Module,
        codegen: &CodeGenerator,
        params: &Vec<LLVMValueRef>,
        ty_info: &mut TypeInfo,
    ) -> LLVMValueRef {
        let mut params_val: Vec<_> =
            self.params
                .into_iter()
                .map(|x| x.code_gen(module, codegen, params, ty_info))
                .collect();
        match self.func.get_ty(ty_info) {
            Type::LambdaType(x) => {
                if let Some(env_ty) = x.env_ty {
                    let lambda = self.func.code_gen(module, codegen, params, ty_info);
                    let lambda_ptr = codegen.build_alloca(type_of(lambda), "");
                    codegen.build_store(lambda, lambda_ptr);
                    let envs_tuple_pointer = codegen.build_struct_gep(lambda_ptr, 0, "");
                    let func_pointer = codegen.build_load(
                        codegen.build_struct_gep(lambda_ptr, 1, ""),
                        "",
                    );
                    let mut envs_val: Vec<_> =
                        env_ty.element_tys.into_iter().enumerate().map(|(idx, x)| {
                            let pointer = codegen.build_struct_gep(envs_tuple_pointer, idx as u32, "");
                            codegen.build_load(pointer, "")
                        }).collect();
                    envs_val.append(&mut params_val);
                    codegen.build_call(func_pointer, envs_val, "")
                } else {
                    let func = self.func.code_gen(module, codegen, params, ty_info);
                    codegen.build_call(func, params_val, "")
                }
            }
            x => panic!("{:?}", x)
        }
    }
}

impl ir::TupleIr {
    fn code_gen(
        self,
        module: &Module,
        codegen: &CodeGenerator,
        params: &Vec<LLVMValueRef>,
        ty_info: &mut TypeInfo,
    ) -> LLVMValueRef {
        let elements_val: Vec<LLVMValueRef> =
            self.elements
                .into_iter()
                .map(|x| x.code_gen(module, codegen, params, ty_info)).collect();
        let ty = struct_type(
            elements_val.iter()
                .map(|x| type_of(*x))
                .collect()
        );
        let val = codegen.build_alloca(ty, "");
        elements_val
            .into_iter()
            .enumerate()
            .for_each(|(id, x)| {
                let ptr =
                    codegen.build_struct_gep(val, id as u32, "");
                codegen.build_store(x, ptr);
            });
        codegen.build_load(val, "ret")
    }
}

impl ir::OpIr {
    fn code_gen(
        self,
        module: &Module,
        codegen: &CodeGenerator,
        params: &Vec<LLVMValueRef>,
        ty_info: &mut TypeInfo,
    ) -> LLVMValueRef {
        let lhs = self.l_expr.code_gen(module, codegen, params, ty_info);
        let rhs = self.r_expr.code_gen(module, codegen, params, ty_info);
        match &self.op as &str {
            "+" => codegen.build_add(lhs, rhs, ""),
            "-" => codegen.build_sub(lhs, rhs, ""),
            "*" => codegen.build_mul(lhs, rhs, ""),
            "/" => {
                let lhs = codegen.build_si_to_fp(lhs, double_type(), "");
                let rhs = codegen.build_si_to_fp(rhs, double_type(), "");
                codegen.build_fp_to_si(codegen.build_fdiv(lhs, rhs, ""), int32_type(), "")
            }
            _ => panic!("error"),
        }
    }
}

impl ir::LambdaIr {
    fn code_gen(
        self,
        module: &Module,
        codegen: &CodeGenerator,
        params: &Vec<LLVMValueRef>,
        ty_info: &mut TypeInfo,
    ) -> LLVMValueRef {
        //ラムダ式の関数作成
        let func_ty = ty_info.look_up_name(self.func_name.clone()).to_llvm_type(false);
        let func = module.get_named_function(&self.func_name);

            if self.env.len() == 0 {
                func.llvm_function
            } else {
                //環境の値取得
                let env_val: Vec<LLVMValueRef> = self.env.into_iter()
                    .map(|x| params[params.len() - x.id - 1])
                    .collect();
                //環境の型生成
                let env_llvm_ty = struct_type(
                    env_val.iter().map(|x| type_of(*x)).collect()
                );


                //ラムダ式の型生成
                let lambda_ty = struct_type(vec![env_llvm_ty, pointer_type(func_ty)]);

                //ラムダ式の生成
                let env_tuple_val = codegen.build_alloca(env_llvm_ty, "");
                let func_val =
                    env_val
                        .into_iter()
                        .enumerate()
                        .for_each(|(id, x)| {
                            let ptr =
                                codegen.build_struct_gep(env_tuple_val, id as u32, "");
                            codegen.build_store(x, ptr);
                        });
                let env_tuple_val = codegen.build_load(env_tuple_val, "");
                let lambda_val = codegen.build_alloca(lambda_ty, "");
                let env_tuple_ptr = codegen.build_struct_gep(lambda_val, 0, "");
                codegen.build_store(env_tuple_val, env_tuple_ptr);
                let func_ptr = codegen.build_struct_gep(lambda_val, 1, "");
                codegen.build_store(func.llvm_function, func_ptr);
                codegen.build_load(lambda_val, "ret")
            }
    }
}
