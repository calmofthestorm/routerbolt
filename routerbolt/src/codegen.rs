use std::rc::Rc;

use crate::*;

#[derive(Clone, Copy, Debug)]
pub enum Backend {
    /// Uses a look up table in the program itself to store the stack.
    Internal,

    /// Uses a memory bank or memory cell to store the stack. Faster and
    /// typically supports larger stack sizes.
    External,
}

#[derive(Clone, Debug)]
pub enum BackendParams {
    Internal(Rc<InternalParams>),
    External(Rc<ExternalParams>),
}

#[derive(Clone, Copy, Debug)]
pub struct InternalParams {
    pub push_entry_size: AddressDelta,
    pub pop_entry_size: AddressDelta,
    pub poke_entry_size: AddressDelta,
    pub push_table_start: Address,
    pub pop_table_start: Address,
    pub poke_table_start: Address,
}

#[derive(Clone, Debug)]
pub struct ExternalParams {
    pub cell_name: Rc<String>,
}

pub fn generate(ir: &IntermediateRepresentation) -> Result<(Vec<String>, Vec<String>)> {
    let mut output = Vec::default();
    let mut annotated = Vec::default();
    let mut instruction_count = 0.into();

    for op in ir.ops().iter() {
        let annotation_start = output.len();

        op.generate(
            ir,
            &mut output,
            Some(&mut annotated),
            &mut instruction_count,
        )?;

        for (j, line) in output[annotation_start..].iter().enumerate() {
            annotated.push(format!("{}\t{}", instruction_count + j.into(), line));
        }

        annotated.push(String::default());

        instruction_count += op.code_size(*ir.backend());
    }

    if let Backend::Internal = ir.backend() {
        generate_internal_stack(
            &ir.stack_config,
            &mut output,
            Some(&mut annotated),
            &mut instruction_count,
        );
    }

    Ok((output, annotated))
}

pub fn generate_internal_stack(
    config: &StackConfig,
    out: &mut Vec<String>,
    mut ann: Option<&mut Vec<String>>,
    ic: &mut Address,
) {
    let size = match config {
        StackConfig::Internal(size) if *size == 0 => {
            return;
        }
        StackConfig::Internal(size) => {
            if let Some(ann) = ann.as_mut() {
                ann.push(format!("\n Begin stack of size {}", size));
            }
            *size
        }
        StackConfig::External(..) => {
            return;
        }
    };

    out.push("end".to_string());
    if let Some(ann) = ann.as_mut() {
        ann.push("// End before stack table (annotations do not show the actual generated stack because it is so long)".to_string());
        ann.push("end".to_string());
        ann.push(String::default());
    }
    *ic += 1.into();

    gen("push", size, out, &mut None, ic, push);
    gen("pop", size, out, &mut None, ic, pop);
    gen("poke", size, out, &mut None, ic, poke);
}

fn gen<F>(
    name: &str,
    stack_size: usize,
    output: &mut Vec<String>,
    annotated: &mut Option<&mut Vec<String>>,
    instruction_count: &mut Address,
    generate_entry: F,
) where
    F: Fn(usize, &mut Vec<String>),
{
    for j in 0..stack_size {
        let start = output.len();

        if let Some(annotated) = annotated.as_mut() {
            annotated.push(format!("// Stack {} table index {}", name, j));
        }

        generate_entry(j, output);

        if let Some(annotated) = annotated.as_mut() {
            for (j, line) in output[start..].iter().enumerate() {
                annotated.push(format!("{}\t{}", *instruction_count + j.into(), line));
            }

            annotated.push(String::default());
        }

        *instruction_count += AddressDelta::from(output.len() - start);
    }
}

fn pop(index: usize, output: &mut Vec<String>) {
    output.push(format!("set MF_acc MF_stack[{}]", index));
    output.push("set @counter MF_resume".to_string());
}

fn poke(index: usize, output: &mut Vec<String>) {
    output.push(format!("set MF_stack[{}] MF_acc", index));
    output.push("set @counter MF_resume".to_string());
}

fn push(index: usize, output: &mut Vec<String>) {
    output.push(format!("set MF_stack[{}] MF_acc", index));
    output.push("op add MF_stack_sz MF_stack_sz 1".to_string());
    output.push("set @counter MF_resume".to_string());
}
