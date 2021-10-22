use crate::*;

/// Runs a Mindustry command directly.
///
/// Destroys:
///   - If stack variables are used: All
///   - If it directly changes any variable starting with `MF_`: that variable
///   - Otherwise: Preserves all
#[derive(Clone, Debug)]
pub struct MindustryOp {
    pub command: MindustryCommand,
}

impl Operation for MindustryOp {
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
            annotated.push(format!("// Mindustry command @{}", output.len()));
        }

        output.push(self.command.to_string());

        Ok(())
    }
}
