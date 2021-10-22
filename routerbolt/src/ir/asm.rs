/// Low-level operations reminiscent of assembly. None of these make use of
/// structured programming constructs, but they do permit low-level procedure
/// calls.
///
/// These make use of a variable named `MF_acc` as the accumulator, and
/// `MF_tmp`/`MF_resume` as scratch space.
use std::rc::Rc;

use crate::*;

/// Pushes return address to the stack and jumps to the target label. RetProc
/// will return from it.
///
/// Destroys: `MF_acc` `MF_tmp` `MF_resume`
#[derive(Clone, Debug)]
pub struct CallProcOp {
    /// Name of label to call.
    pub target: LabelName,
}

impl Operation for CallProcOp {
    fn code_size(&self, backend: Backend) -> AddressDelta {
        match backend {
            Backend::Internal => 5,
            Backend::External => 4,
        }
        .into()
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!(
                "// CallProc {} @{}",
                self.target.as_ref(),
                output.len()
            ));
        }

        let target = ir.labels()[&self.target];

        match ir.backend_params() {
            BackendParams::Internal(int) => {
                output.push("op add MF_acc @counter 4".to_string());
                output.push("op add MF_resume @counter 2".to_string());
                output.push(format!("op mul MF_tmp {} MF_stack_sz", int.push_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.push_table_start));
                output.push(format!("set @counter {}", target));
            }
            BackendParams::External(ext) => {
                output.push("op add MF_acc @counter 3".to_string());
                output.push(format!("write MF_acc {} MF_stack_sz", ext.cell_name));
                output.push("op add MF_stack_sz MF_stack_sz 1".to_string());
                output.push(format!("set @counter {}", target));
            }
        }

        Ok(())
    }
}

/// Pops the top of the stack, and jumps to that address. Used with
/// `CallProcOp`.
///
/// Destroys: `MF_acc` `MF_tmp` `MF_resume`
#[derive(Clone, Debug)]
pub struct RetProcOp {}

impl Operation for RetProcOp {
    fn code_size(&self, backend: Backend) -> AddressDelta {
        match backend {
            Backend::Internal => 5,
            Backend::External => 2,
        }
        .into()
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!("// Ret @{}", output.len()));
        }

        match ir.backend_params() {
            BackendParams::Internal(int) => {
                output.push("op sub MF_stack_sz MF_stack_sz 1".to_string());
                output.push("op add MF_resume @counter 2".to_string());
                output.push(format!("op mul MF_tmp {} MF_stack_sz", int.pop_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.pop_table_start));
                output.push(format!("set @counter MF_acc"));
            }
            BackendParams::External(ext) => {
                output.push("op sub MF_stack_sz MF_stack_sz 1".to_string());
                output.push(format!("read @counter {} MF_stack_sz", ext.cell_name));
            }
        }

        Ok(())
    }
}

/// Pushes `MF_acc` to the stack.
///
/// Destroys: `MF_tmp` `MF_resume`
/// Preserves: `MF_acc`
#[derive(Clone, Debug)]
pub struct PushOp {}

impl Operation for PushOp {
    fn code_size(&self, backend: Backend) -> AddressDelta {
        match backend {
            Backend::Internal => 3,
            Backend::External => 2,
        }
        .into()
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!("// Push @{}", output.len()));
        }

        match ir.backend_params() {
            BackendParams::Internal(int) => {
                output.push("op add MF_resume @counter 2".to_string());
                output.push(format!("op mul MF_tmp {} MF_stack_sz", int.push_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.push_table_start));
            }
            BackendParams::External(ext) => {
                output.push(format!("write MF_acc {} MF_stack_sz", ext.cell_name));
                output.push("op add MF_stack_sz MF_stack_sz 1".to_string());
            }
        }

        Ok(())
    }
}

/// Pops the top of the stack into `MF_acc`.
///
/// Destroys: `MF_tmp` `MF_resume`
/// Returns: `MF_acc`
#[derive(Clone, Debug)]
pub struct PopOp {}

impl Operation for PopOp {
    fn code_size(&self, backend: Backend) -> AddressDelta {
        match backend {
            Backend::Internal => 4,
            Backend::External => 2,
        }
        .into()
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!("// Pop @{}", output.len()));
        }

        match ir.backend_params() {
            BackendParams::Internal(int) => {
                output.push("op sub MF_stack_sz MF_stack_sz 1".to_string());
                output.push("op add MF_resume @counter 2".to_string());
                output.push(format!("op mul MF_tmp {} MF_stack_sz", int.pop_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.pop_table_start));
            }
            BackendParams::External(ext) => {
                output.push("op sub MF_stack_sz MF_stack_sz 1".to_string());
                output.push(format!("read MF_acc {} MF_stack_sz", ext.cell_name));
            }
        }

        Ok(())
    }
}

/// Copies the stack entry `depth` places from the top into `MF_acc`.
/// Specifying `depth=0` will get the top of the stack.
///
/// Destroys: `MF_tmp` `MF_resume`
/// Returns: `MF_acc`
#[derive(Clone, Debug)]
pub struct PeekOp {
    pub depth: MindustryTerm,
}

impl Operation for PeekOp {
    fn code_size(&self, backend: Backend) -> AddressDelta {
        match (backend, self.depth.as_ref().parse::<usize>()) {
            (Backend::Internal, Ok(..)) => 4,
            (Backend::Internal, Err(..)) => 5,
            (Backend::External, Ok(..)) => 2,
            (Backend::External, Err(..)) => 3,
        }
        .into()
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!("// Peek depth {} @{}", self.depth, output.len()));
        }

        match self.depth.as_ref().parse::<usize>() {
            Ok(literal_number) => {
                output.push(format!("op sub MF_tmp MF_stack_sz {}", literal_number + 1));
            }
            Err(..) => {
                output.push(format!("op sub MF_tmp MF_stack_sz {}", self.depth));
                output.push(format!("op sub MF_tmp MF_tmp {}", 1));
            }
        }

        match ir.backend_params() {
            BackendParams::Internal(int) => {
                // Not an error -- peek and pop use the same table.
                output.push("op add MF_resume @counter 2".to_string());
                output.push(format!("op mul MF_tmp {} MF_tmp", int.pop_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.pop_table_start));
            }
            BackendParams::External(ext) => {
                output.push(format!("read MF_acc {} MF_tmp", ext.cell_name));
            }
        }

        Ok(())
    }
}

/// Copies `MF_acc` into the stack entry `depth` places from the top. Specifying
/// `depth=0` will use the top of the stack.
///
/// Destroys: `MF_tmp` `MF_resume`
#[derive(Clone, Debug)]
pub struct PokeOp {
    pub depth: MindustryTerm,
}

impl Operation for PokeOp {
    fn code_size(&self, backend: Backend) -> AddressDelta {
        match (backend, self.depth.as_ref().parse::<usize>()) {
            (Backend::Internal, Ok(..)) => 4,
            (Backend::Internal, Err(..)) => 5,
            (Backend::External, Ok(..)) => 2,
            (Backend::External, Err(..)) => 3,
        }
        .into()
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!("// Poke depth {} @{}", self.depth, output.len()));
        }

        match self.depth.as_ref().parse::<usize>() {
            Ok(literal_number) => {
                output.push(format!("op sub MF_tmp MF_stack_sz {}", literal_number + 1));
            }
            Err(..) => {
                output.push(format!("op sub MF_tmp MF_stack_sz {}", self.depth));
                output.push(format!("op sub MF_tmp MF_tmp {}", 1));
            }
        }

        match ir.backend_params() {
            BackendParams::Internal(int) => {
                output.push("op add MF_resume @counter 2".to_string());
                output.push(format!("op mul MF_tmp {} MF_tmp", int.poke_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.poke_table_start));
            }
            BackendParams::External(ext) => {
                output.push(format!("write MF_acc {} MF_tmp", ext.cell_name));
            }
        }

        Ok(())
    }
}

/// Sets `dest` to `source`.
///
/// Preserves: All
#[derive(Clone, Debug)]
pub struct SetOp {
    source: MindustryTerm,
    dest: MindustryTerm,
}

impl SetOp {
    pub fn new(dest: MindustryTerm, source: MindustryTerm) -> SetOp {
        SetOp { source, dest }
    }
}

impl Operation for SetOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        1.into()
    }

    fn generate(
        &self,
        _ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!(
                "// Set {} {} @{}",
                &self.dest,
                &self.source,
                output.len()
            ));
        }

        output.push(format!("set {} {}", &self.dest, &self.source));

        Ok(())
    }
}

/// Defines a label that may be used with `JumpOp` and `CallProcOp`.
///
/// Preserves: All
#[derive(Clone, Debug)]
pub struct LabelOp {
    pub target: LabelName,
}

impl Operation for LabelOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        0.into()
    }

    fn generate(
        &self,
        _ir: &IntermediateRepresentation,
        _output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!("{}:", self.target.as_ref()));
        }

        Ok(())
    }
}

/// Jumps to the specified label. This is identical to Mindustry's built-in
/// jump, except that a label is specified for the first argument instead of the
/// line number.
///
/// Preserves: All
#[derive(Clone, Debug)]
pub struct JumpOp {
    pub target: LabelName,
    pub condition: Condition,
}

impl Operation for JumpOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        1.into()
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!(
                "// Jump: {} {} @{}",
                &self.target,
                &self.condition,
                output.len()
            ));
        }

        output.push(format!(
            "jump {} {}",
            ir.labels()[&self.target],
            self.condition,
        ));

        Ok(())
    }
}

/// Does a built-in operation as per Mindustry `op`.
///
/// Preserves: All
#[derive(Clone, Debug)]
pub struct MathOp {
    pub operation: Rc<String>,
    pub dest: MindustryTerm,
    pub arg1: MindustryTerm,
    pub arg2: MindustryTerm,
}

impl Operation for MathOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        1.into()
    }

    fn generate(
        &self,
        _ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        if let Some(annotated) = annotated {
            annotated.push(format!(
                "// Op (the Mindustry one): {} {} {} {} @{}",
                &self.operation,
                &self.dest,
                &self.arg1,
                &self.arg2,
                output.len()
            ));
        }

        output.push(format!(
            "op {} {} {} {}",
            &self.operation, &self.dest, &self.arg1, &self.arg2,
        ));

        Ok(())
    }
}
