/// Variables that live on the stack, in the body of a function. To distinguish
/// them from Mindustry variables, we always begin them with *.
///
/// There are only two scopes in this language: global and function body. Stack
/// variables are only allowed (compile error) inside a function body. Unlike
/// global variables (which are just the Mindustry ones), they are tied to a
/// particular invocation ("frame") of the function, so, e.g., each call to a
/// recursive function will have its own instance of its stack variables.
///
/// Note that although loops, if statements, etc use {} as syntax, they do not
/// create a scope -- only functions do. This simplifies look up, since we use
/// different syntax for stack variables, there is no need to search enclosing
/// scopes, and without RAII the value of such scoping is limited.
///
/// It would be possible to add such scoping later, but I would recommend only
/// doing so if we implement a recursive parser into an AST, as it will make
/// control flow more complicated (crossing definitions with jump,
/// break/continue in loops, etc).
use crate::*;

/// Declares a function-scope variable stored on the stack. Variables must be
/// declared before use.
///
/// e.g.: `let *my_var`
///
/// Note that because there is only function scope, this is legal:
///
/// if equal a 5 {
///   let *my_var
/// }
///
/// set *my_var 10
///
/// Destroys: None
#[derive(Clone, Debug)]
pub struct LetOp {
    pub name: StackVar,
    pub pos: FrameIndex,
}

impl Operation for LetOp {
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
            annotated.push(format!(
                "// Let {} (stack offset {}) @{}",
                self.name,
                self.pos,
                output.len()
            ));
        }

        Ok(())
    }
}

/// Gets the value of a stack variable.
///
/// e.g.: `set mindustry_var *my_var`
///
/// Destroys: All
#[derive(Clone, Debug)]
pub struct GetStackOp {
    pub global: MindustryTerm,
    pub stack: StackVar,
    pub function: FunctionName,
}

impl Operation for GetStackOp {
    fn code_size(&self, backend: Backend) -> AddressDelta {
        match backend {
            Backend::Internal if self.global.as_ref() != "MF_acc" => 5,
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
            annotated.push(format!(
                "// GetStack {} {} in fn {} @{}",
                self.global.as_ref(),
                self.stack,
                self.function.as_ref(),
                output.len()
            ));
        }

        let depth = ir.functions()[&self.function].stack_var_depth(&self.stack)?;

        match ir.backend_params() {
            BackendParams::Internal(int) => {
                output.push("op add MF_resume @counter 3".to_string());
                output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                output.push(format!("op mul MF_tmp {} MF_tmp", int.pop_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.pop_table_start));
                if self.global.as_ref() != "MF_acc" {
                    output.push(format!("set {} MF_acc", self.global.as_ref()));
                }
            }
            BackendParams::External(ext) => {
                output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                output.push(format!("read {} {} MF_tmp", self.global, ext.cell_name));
            }
        }

        Ok(())
    }
}

/// Sets the value of a stack variable.
///
/// e.g.: `set *my_var mindustry_var`
///
/// Destroys: All
#[derive(Clone, Debug)]
pub struct SetStackOp {
    pub global: MindustryTerm,
    pub stack: StackVar,
    pub function: FunctionName,
}

impl Operation for SetStackOp {
    fn code_size(&self, backend: Backend) -> AddressDelta {
        match backend {
            Backend::Internal if self.global.as_ref() != "MF_acc" => 5,
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
            annotated.push(format!(
                "// SetStack {} {} in fn {} @{}",
                self.stack,
                self.global.as_ref(),
                self.function.as_ref(),
                output.len()
            ));
        }

        let depth = ir.functions()[&self.function].stack_var_depth(&self.stack)?;
        let depth: usize = depth.into();

        match ir.backend_params() {
            BackendParams::Internal(int) => {
                if self.global.as_ref() != "MF_acc" {
                    output.push(format!("set MF_acc {}", self.global.as_ref()));
                }
                output.push("op add MF_resume @counter 3".to_string());
                output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                output.push(format!("op mul MF_tmp {} MF_tmp", int.poke_entry_size));
                output.push(format!("op add @counter {} MF_tmp", int.poke_table_start));
            }
            BackendParams::External(ext) => {
                output.push(format!("op sub MF_tmp MF_stack_sz {}", depth));
                output.push(format!("write {} {} MF_tmp", self.global, ext.cell_name));
            }
        }

        Ok(())
    }
}
