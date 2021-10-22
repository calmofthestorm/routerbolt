use std::collections::HashMap;

use std::rc::Rc;

use crate::*;

#[derive(Debug)]
pub enum StackConfig {
    Internal(usize),
    External(Rc<String>),
}

#[derive(Debug)]
pub struct IntermediateRepresentation {
    pub ops: Vec<IrOp>,
    pub stack_config: StackConfig,
    pub labels: HashMap<LabelName, Address>,
    pub functions: HashMap<FunctionName, Rc<FunctionOp>>,
    pub backend: Backend,
    pub backend_params: BackendParams,
}

impl IntermediateRepresentation {
    pub fn parse(text: &str) -> Result<IntermediateRepresentation> {
        parser::parse(text)
    }

    pub fn generate(&self) -> Result<(Vec<String>, Vec<String>)> {
        generate(self)
    }

    pub fn ops(&self) -> &Vec<IrOp> {
        &self.ops
    }

    pub fn functions(&self) -> &HashMap<FunctionName, Rc<FunctionOp>> {
        &self.functions
    }

    pub fn labels(&self) -> &HashMap<LabelName, Address> {
        &self.labels
    }

    pub fn backend_params(&self) -> &BackendParams {
        &self.backend_params
    }

    pub fn backend(&self) -> &Backend {
        &self.backend
    }
}

/// Generates the IR to read `source` and write its value to `dest`, where
/// either/both may be on the stack.
pub fn ir_copy_arg(
    dest: Term,
    source: Term,
    function: &Option<FunctionName>,
) -> Result<IrSequence> {
    match (source, dest, function.as_ref()) {
        (Term::Mindustry(source), Term::Mindustry(dest), _) => {
            Ok(IrOp::Set(SetOp::new(dest, source)).into())
        }
        (Term::StackVar(stack_source), Term::Mindustry(dest), Some(function)) => {
            let op = GetStackOp {
                global: dest,
                stack: stack_source,
                function: function.clone(),
            };
            Ok(IrOp::GetStack(op).into())
        }
        (Term::Mindustry(source), Term::StackVar(stack_dest), Some(function)) => {
            let op = SetStackOp {
                global: source,
                stack: stack_dest,
                function: function.clone(),
            };
            Ok(IrOp::SetStack(op).into())
        }
        (Term::StackVar(stack_source), Term::StackVar(stack_dest), Some(function)) => {
            let acc = MindustryTerm::accumulator();
            let op1 = GetStackOp {
                global: acc.clone(),
                stack: stack_source,
                function: function.clone(),
            };
            let op1 = IrOp::GetStack(op1);

            let op2 = SetStackOp {
                global: acc,
                stack: stack_dest,
                function: function.clone(),
            };
            let op2 = IrOp::SetStack(op2);
            Ok((op1, op2).into())
        }
        _ => {
            bail!("Stack variables (start with *) may not be used outside a fuction");
        }
    }
}

pub fn ir_read_two_write_one(
    dest: Term,
    arg1: Term,
    arg2: Term,
    function: &Option<FunctionName>,
) -> Result<(
    IrSequence,
    MindustryTerm,
    MindustryTerm,
    MindustryTerm,
    IrSequence,
)> {
    let (read, arg1, arg2) = if arg1 == arg2 {
        let (read, arg) = ir_read_one_arg(arg1, function)?;
        (read, arg.clone(), arg)
    } else {
        ir_read_two_args(arg1, arg2, function)?
    };

    let (dest, write) = ir_write_one(dest, function)?;

    Ok((read, dest, arg1, arg2, write))
}

pub fn ir_write_one(
    dest: Term,
    function: &Option<FunctionName>,
) -> Result<(MindustryTerm, IrSequence)> {
    match (dest, function.as_ref()) {
        (Term::Mindustry(dest), _) => Ok((dest, None.into())),
        (Term::StackVar(stack_dest), Some(function)) => {
            let acc = MindustryTerm::accumulator();
            let op = SetStackOp {
                global: acc.clone(),
                stack: stack_dest,
                function: function.clone(),
            };
            Ok((acc, IrOp::SetStack(op).into()))
        }
        _ => {
            bail!("Stack variables (start with *) may not be used outside a fuction");
        }
    }
}

/// Generates the IR to read one argument, potentially on the stack.
pub fn ir_read_one_arg(
    arg: Term,
    function: &Option<FunctionName>,
) -> Result<(IrSequence, MindustryTerm)> {
    match (arg, function.as_ref()) {
        (Term::Mindustry(arg), _) => Ok((None.into(), arg)),
        (Term::StackVar(stack_arg), Some(function)) => {
            let arg = MindustryTerm::accumulator();
            let op = GetStackOp {
                global: arg.clone(),
                stack: stack_arg,
                function: function.clone(),
            };
            Ok((IrOp::GetStack(op).into(), arg))
        }
        _ => {
            bail!("Stack variables (start with *) may not be used outside a fuction");
        }
    }
}

/// Generates the IR to read two arguments (potentially on the stack).
pub fn ir_read_two_args(
    arg1: Term,
    arg2: Term,
    function: &Option<FunctionName>,
) -> Result<(IrSequence, MindustryTerm, MindustryTerm)> {
    if arg1 == arg2 {
        let (seq, arg) = ir_read_one_arg(arg1, function)?;
        return Ok((seq, arg.clone(), arg));
    }

    match (arg1, arg2, function.as_ref()) {
        (Term::Mindustry(arg1), Term::Mindustry(arg2), _) => Ok((None.into(), arg1, arg2)),
        (Term::StackVar(arg1s), Term::Mindustry(arg2), Some(function)) => {
            let arg1 = MindustryTerm::accumulator();
            let op = GetStackOp {
                global: arg1.clone(),
                stack: arg1s,
                function: function.clone(),
            };
            Ok((IrOp::GetStack(op).into(), arg1, arg2))
        }
        (Term::Mindustry(arg1), Term::StackVar(arg2s), Some(function)) => {
            let arg2 = MindustryTerm::accumulator();
            let op = GetStackOp {
                global: arg2.clone(),
                stack: arg2s,
                function: function.clone(),
            };
            Ok((IrOp::GetStack(op).into(), arg1, arg2))
        }
        (Term::StackVar(arg1s), Term::StackVar(arg2s), Some(function)) => {
            let arg1 = MindustryTerm::stack_tmp();
            let arg2 = MindustryTerm::accumulator();

            let op1 = GetStackOp {
                global: arg1.clone(),
                stack: arg1s,
                function: function.clone(),
            };
            let op1 = IrOp::GetStack(op1);

            // Careful -- `GetStackOp` uses the accumulator, so the second op
            // to be emitted must be the one that sets it.
            let op2 = GetStackOp {
                global: arg2.clone(),
                stack: arg2s,
                function: function.clone(),
            };
            let op2 = IrOp::GetStack(op2);
            Ok(((op1, op2).into(), arg1, arg2))
        }
        _ => {
            bail!("Stack variables (start with *) may not be used outside a fuction");
        }
    }
}
