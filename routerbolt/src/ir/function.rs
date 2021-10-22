use std::collections::HashMap;
use std::convert::TryFrom;

use crate::*;

/// Defines a function.
///
/// Functions define the only scopes in this language. Their arguments are tied
/// to the scope, as are any let declarations in the function body. Stack
/// variables may only be used in a function.
///
/// FIXME: Implement this restriction. At present, they can be defined anywhere
/// and fallthrough is undetected UB.
///
/// At present, function definitions must occur immediately following either
/// `end` or `return`. This is because we don't really parse the AST, and so it
/// would otherwise be possible to "fall through" into a function body,
/// resulting in stack corruption. These two instructions are sufficient but not
/// necessary to prevent this. We could in theory hard code other special cases
/// such as "jump always", but without full control flow analysis it seems
/// sufficient to simply place function definitions at the end of the program
/// after `end`.
#[derive(Clone, Debug)]
pub struct FunctionOp {
    // Function name. Must be unique.
    pub name: FunctionName,

    // Argument names the function takes. All must start with *, as they are
    // stack vars.
    pub args: Vec<StackVar>,

    // Return value names. Only the number matters once we validate them, but we
    // save the names for annotation.
    pub returns: Vec<Term>,

    // The local variables (including args) offset from the base pointer. Since
    // we don't have a base pointer, we use that to calculate the offset from
    // the stack size using `stack_var_depth`.
    pub locals: HashMap<StackVar, FrameIndex>,

    // The offset in instructions of the function body. Set later, hence option.
    pub address: Option<Address>,
}

impl FunctionOp {
    pub fn stack_var_depth(&self, name: &StackVar) -> Result<StackDepth> {
        let offset = self.locals.get(name).with_context(|| {
            format!(
                "innermost function definition does not have let variable named '{}'",
                name
            )
        })?;

        // Position relative to stack size. Consider that the 0th
        // argument has offset 0, but is "deepest" in the stack.
        //
        // Also, the stack size is a size, and we want an index.
        let offset: usize = offset.into();
        Ok((self.locals.len() - offset).into())
    }

    pub fn declare(
        name: FunctionName,
        arg_names: &[&str],
        return_names: &[&str],
    ) -> Result<FunctionOp> {
        let mut locals: HashMap<StackVar, FrameIndex> = HashMap::new();

        let mut args = Vec::with_capacity(arg_names.len());

        // All args to a function are stack variables.
        for (j, arg) in arg_names.into_iter().enumerate() {
            let arg = StackVar::try_from(*arg)
                .with_context(|| format!("function {} argument {} name \"{}\"", &name, j, &arg))?;
            if locals.insert(arg.clone(), locals.len().into()).is_some() {
                bail!(
                    "function {} argument {} duplicate name \"{}\"",
                    &name,
                    j,
                    &arg
                );
            }
            args.push(arg);
        }

        let mut returns = Vec::with_capacity(return_names.len());

        // Returned value names are mostly ignored here -- we only care that the
        // number match and they not be duplicated. In particular, we permit the
        // * for global vs local, but it has no effect except documentation. Two
        // different return statements may return a global vs a local for the
        // same value, and the caller is free to bind it to either as well.
        for (j, ret) in return_names.into_iter().enumerate() {
            let ret = Term::try_from(ret.clone())
                .with_context(|| format!("function {} return value {} name {}", &name, j, &ret))?;
            if returns.contains(&ret) {
                bail!(
                    "function {} return value {} duplicate name {}",
                    &name,
                    j,
                    &ret
                );
            }
            returns.push(ret);
        }

        let f = FunctionOp {
            name,
            args,
            returns,
            locals,
            address: None,
        };

        Ok(f)
    }

    pub fn start_parse(&mut self, address: Address) {
        let set = self.address.replace(address);
        assert!(set.is_none());
    }
}

impl Operation for FunctionOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        0.into()
    }

    fn generate(
        &self,
        _ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format_arrow_annotation(
                "// Function {} @{}",
                &self.name,
                &self.args,
                &self.returns,
                output.len(),
            ));
        }

        Ok(())
    }
}

/// Returns from a `CallOp` to a function defined with a `FunctionOp`.
///
/// FIXME: At present, explicit return is required from all functions, and
/// failure to do so is undefined behavior. I'd like to fix this, but it's hard
/// to do control-flow analysis without an AST. It would probably be possible to
/// put together something for this special case, but would be a fair bit of
/// work toward making all blocks scopes anyway.
///
/// e.g.:
/// `return`
/// `return 5 7 v1 *v2`
///
/// Destroys: `MF_acc` `MF_tmp` `MF_resume`
#[derive(Clone, Debug)]
pub struct ReturnOp {
    // Name of the function this is a return from.
    pub function: FunctionName,

    // The values being returned.
    pub values: Vec<Term>,

    pub size: AddressDelta,
}

// FIXME: Can probably re-arrange stack math to use fewer instructions.
impl ReturnOp {
    pub fn new(function: &FunctionOp, value_names: &[&str], backend: Backend) -> Result<ReturnOp> {
        let mut total = 0;
        let mut values = Vec::with_capacity(value_names.len());

        if value_names.len() != function.returns.len() {
            bail!(
                "function specifies {} return values but return statement has {}",
                function.returns.len(),
                value_names.len(),
            );
        }

        for (j, value) in value_names.iter().copied().enumerate() {
            let value =
                Term::try_from(value).with_context(|| format!("return value {} '{}'", j, value))?;
            total += match &value {
                Term::StackVar(..) => match backend {
                    Backend::Internal => 5,
                    Backend::External => 2,
                },
                Term::Mindustry(..) => 1,
            };
            values.push(value);
        }

        // Remove locals and return address from the stack.
        total += 1;

        // Pop return address and return.
        total += match backend {
            Backend::Internal => 4,
            Backend::External => 1,
        };

        Ok(ReturnOp {
            function: function.name.clone(),
            values,
            size: total.into(),
        })
    }
}

impl Operation for ReturnOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        self.size
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format_return_annotation(self, output.len()));
        }

        let function = &ir.functions()[&self.function];
        if self.values.len() != function.returns.len() {
            bail!(
                "function {} specifies {} return values but return statement has {}",
                &self.function,
                self.values.len(),
                function.returns.len()
            );
        }

        for (j, arg) in self.values.iter().enumerate() {
            match arg {
                Term::StackVar(arg) => {
                    let depth = function.stack_var_depth(&arg)?;

                    match ir.backend_params() {
                        BackendParams::Internal(int) => {
                            output.push("op add MF_resume @counter 3".to_string());
                            output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                            output.push(format!("op mul MF_tmp {} MF_tmp", int.pop_entry_size));
                            output.push(format!("op add @counter {} MF_tmp", int.pop_table_start));

                            output.push(format!("set MF_ret{} MF_acc", j));
                        }
                        BackendParams::External(ext) => {
                            output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                            output.push(format!("read MF_ret{} {} MF_tmp", j, ext.cell_name));
                        }
                    }
                }
                Term::Mindustry(..) => {
                    output.push(format!("set MF_ret{} {}", j, arg));
                }
            }
        }

        // Remove locals and return address from the stack.
        output.push(format!(
            "op sub MF_stack_sz MF_stack_sz {}",
            1 + function.locals.len()
        ));

        match ir.backend_params() {
            BackendParams::Internal(int) => {
                // Same as `Ret`, except that we roll in the sub to stack size as above.
                output.push("op add MF_resume @counter 2".to_string());
                output.push(format!("op mul MF_tmp {} MF_stack_sz", int.pop_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.pop_table_start));
                output.push(format!("set @counter MF_acc"));
            }
            BackendParams::External(ext) => {
                output.push(format!("read @counter {} MF_stack_sz", ext.cell_name));
            }
        }

        Ok(())
    }
}

/// Calls the specified `FunctionOp` with the given arguments. Stack variables
/// may be used with *, or any Mindustry expression (variable or literal)
/// without.
///
/// e.g.: `call foobar "hello" *a b -> ret1 *ret2`
///
/// Destroys: All
#[derive(Clone, Debug)]
pub struct CallOp {
    // The name of the function this call is being made from, if in one. Used to
    // access stack variables, which may be used when a call is made within a
    // function.
    pub call_site_function: Option<FunctionName>,

    // The name of the function to call.
    pub target_function: FunctionName,

    // The arguments and returns. These may start with * for a stack var, or
    // otherwise be a Mindustry term.
    pub args: Vec<Term>,
    pub returns: Vec<Term>,

    // The number of instructions up to and including the actual jump to the
    // target function entry point.
    pub before_call_size: AddressDelta,

    // The total number of instructions generated by the call op.
    pub total_size: AddressDelta,
}

impl CallOp {
    pub fn new(
        args: Vec<Term>,
        returns: Vec<Term>,
        target_function_num_locals: usize,
        target_function: FunctionName,
        call_site_function: Option<FunctionName>,
        backend: Backend,
    ) -> CallOp {
        // Size before (and including) the actual call.
        let mut before_call_size = 0.into();

        // Push return address
        before_call_size += match backend {
            Backend::Internal => 4,
            Backend::External => 3,
        }
        .into();

        for arg in args.iter() {
            before_call_size += match (backend, arg) {
                (Backend::Internal, Term::StackVar(..)) => 7,
                (Backend::Internal, Term::Mindustry(..)) => 4,
                (Backend::External, Term::StackVar(..)) => 4,
                (Backend::External, Term::Mindustry(..)) => 2,
            }
            .into();
        }

        // Extra local variables (other than args) must increase stack pointer.
        if target_function_num_locals != args.len() {
            before_call_size += 1.into();
        }

        // Jump to function entry point
        before_call_size += 1.into();

        // Total size including the code after the call to process return
        // variables.
        let mut total_size = before_call_size;

        for arg in returns.iter() {
            total_size += match (backend, arg) {
                (Backend::Internal, Term::StackVar(..)) => 5,
                (Backend::Internal, Term::Mindustry(..)) => 1,
                (Backend::External, Term::StackVar(..)) => 2,
                (Backend::External, Term::Mindustry(..)) => 1,
            }
            .into();
        }

        CallOp {
            target_function,
            call_site_function,
            args,
            returns,
            before_call_size,
            total_size,
        }
    }
}

// FIXME: Can probably re-arrange stack math to use fewer instructions.
impl Operation for CallOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        self.total_size.into()
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format_arrow_annotation(
                "// Call",
                &self.target_function,
                &self.args,
                &self.returns,
                output.len(),
            ));
        }

        let func = match ir.functions().get(&self.target_function) {
            Some(func) => func,
            None => bail!("function {} is not found", &self.target_function),
        };

        if self.returns.len() != func.returns.len() {
            bail!(
                "call site specifies {} return values but function {} returns {} values",
                self.returns.len(),
                &func.name,
                func.returns.len()
            );
        }

        if self.args.len() != func.args.len() {
            bail!(
                "Call site specifies {} arguments but function {} takes {} arguments",
                self.args.len(),
                &func.name,
                func.args.len()
            );
        }

        // Push the return address. This is the cleanup code after
        // the call site.
        match ir.backend_params() {
            BackendParams::Internal(int) => {
                output.push(format!(
                    "op add MF_acc @counter {}",
                    self.before_call_size - 1.into()
                ));
                output.push("op add MF_resume @counter 2".to_string());
                output.push(format!("op mul MF_tmp {} MF_stack_sz", int.push_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.push_table_start));
            }
            BackendParams::External(ext) => {
                output.push(format!(
                    "op add MF_acc @counter {}",
                    self.before_call_size - 1.into()
                ));
                output.push(format!("write MF_acc {} MF_stack_sz", ext.cell_name));
                output.push("op add MF_stack_sz MF_stack_sz 1".to_string());
            }
        }

        for (j, arg) in self.args.iter().enumerate() {
            match arg {
                Term::StackVar(arg) => {
                    let call_site_function = self
                        .call_site_function
                        .as_ref()
                        .context("Internal error: forward reference")?;
                    let depth = ir.functions()[call_site_function].stack_var_depth(&arg)?;

                    // We have been pushing to the stack, so the value
                    // we target is being pushed down (we don't use a
                    // frame pointer, so this is all relative to the
                    // stack size).
                    let mut depth: usize = depth.into();
                    depth += j + 1;

                    // Peek then push.
                    match ir.backend_params() {
                        BackendParams::Internal(int) => {
                            output.push("op add MF_resume @counter 3".to_string());
                            output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                            output.push(format!("op mul MF_tmp {} MF_tmp", int.pop_entry_size));
                            output.push(format!("op add @counter {} MF_tmp", int.pop_table_start));

                            output.push("op add MF_resume @counter 2".to_string());
                            output
                                .push(format!("op mul MF_tmp {} MF_stack_sz", int.push_entry_size));
                            output.push(format!("op add @counter {} MF_tmp", int.push_table_start));
                        }
                        BackendParams::External(ext) => {
                            output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                            output.push(format!("read MF_acc {} MF_tmp", ext.cell_name));
                            output.push(format!("write MF_acc {} MF_stack_sz", ext.cell_name));
                            output.push("op add MF_stack_sz MF_stack_sz 1".to_string());
                        }
                    }
                }
                Term::Mindustry(..) => match ir.backend_params() {
                    BackendParams::Internal(int) => {
                        output.push(format!("set MF_acc {}", arg));
                        output.push("op add MF_resume @counter 2".to_string());
                        output.push(format!("op mul MF_tmp {} MF_stack_sz", int.push_entry_size));
                        output.push(format!("op add @counter {} MF_tmp", int.push_table_start));
                    }
                    BackendParams::External(ext) => {
                        output.push(format!("write {} {} MF_stack_sz", arg, ext.cell_name));
                        output.push("op add MF_stack_sz MF_stack_sz 1".to_string());
                    }
                },
            }
        }

        // Reserve room on the stack for any stack variables in
        // addition to the args.
        let additional = func.locals.len() - func.args.len();
        if additional > 0 {
            output.push(format!("op add MF_stack_sz MF_stack_sz {}", additional));
        }

        // Jump to the function entry point.
        // Optimization: The final push above could jump directly to
        // the destination.
        output.push(format!(
            "jump {} always x false",
            func.address
                .context("Internal error: Forward reference")?
                .as_ref()
        ));

        // The function's Return should have popped the args and
        // return address off the stack, and placed the return args
        // into MF_ret<n>.
        //
        // Now we need to map the returned args into the destination
        // requested.
        //
        // NOTE: We could do a direct stack-to-stack transfer of return
        // variables if we made global/local part of the function's call
        // signature rather than having everything go through MF_ret.
        for (j, arg) in self.returns.iter().enumerate() {
            match arg {
                Term::StackVar(arg) => {
                    let call_site_function = self
                        .call_site_function
                        .as_ref()
                        .context("Internal error: Forward refeerence")?;
                    let depth = ir.functions()[call_site_function].stack_var_depth(&arg)?;

                    match ir.backend_params() {
                        BackendParams::Internal(int) => {
                            output.push("op add MF_resume @counter 4".to_string());
                            output.push(format!("set MF_acc MF_ret{}", j));
                            output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                            output.push(format!("op mul MF_tmp {} MF_tmp", int.poke_entry_size));
                            output.push(format!("op add @counter {} MF_tmp", int.poke_table_start));
                        }
                        BackendParams::External(ext) => {
                            output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                            output.push(format!("write MF_ret{} {} MF_tmp", j, ext.cell_name));
                        }
                    }
                }
                Term::Mindustry(..) => {
                    output.push(format!("set {} MF_ret{}", arg, j));
                }
            }
        }

        Ok(())
    }
}
