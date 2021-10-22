use crate::*;

/// Begins an if statement. Only a single condition is supported at the moment,
/// and may be any Mindustry "jump" arguments. Else works, but else is not
/// implemented.
///
/// Since we don't parse an AST, if statements are simply desugared into a
/// sequence of instructions and jumps. This also creates inconveniences such as
/// the need to negate the condition to jump in code generation.
///
/// The good news is that they can be composed with themselves and other control
/// flow structures as expected and arbitrarily deeply.
///
/// The bad news is what we currently generate is inefficient and confusing:
///
/// `if <foo> { <bar> } else { <qux> }` generates:
///
/// 0: jump 2 <foo>
/// 1: jump 4 always
/// 2: <bar>
/// 3: jump 5 always
/// 4: <qux>
///
/// FIXME: We just need to negate conditions (or reorder statements) and we can generate:
///
/// 0: jump 3 <foo>
/// 1: <qux>
/// 2: jump 4 always
/// 3: <bar>
///
/// or
///
/// 0: jump 3 !<foo>
/// 1: <bar>
/// 2: jump 4 always
/// 3: <qux>
///
/// In code, you could write:
/// if greaterThan x 5 {
/// ...
/// }
///
/// Or
/// if <cond> {
/// ...
/// } else {
/// ...
/// }
///
/// Note that "} else {", "if ... {", and "}" are parsed as single ops, and must
/// each be on its own line. Our parsing is firmly unstructured, despite the
/// sugar.
///
/// Desugar to:
///   - With else: `IfOp` ... `ElseOp` ... `}`
///   - Without else: `IfOp` ... `}`
///
/// Preserves: All if no stack vars are used in the condition, otherwise None.
#[derive(Clone, Debug)]
pub struct IfOp {
    condition: Condition,

    // The first address after the end of the "true" branch. This will be the
    // first address in the else clause if present.
    end: Option<Address>,
}

impl IfOp {
    pub fn new(condition: Condition) -> IfOp {
        let end = None;
        IfOp { condition, end }
    }

    pub fn resolve_forward(&mut self, end: Address) {
        let set = self.end.replace(end);
        assert!(set.is_none());
    }
}

impl Operation for IfOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        // Two instructions for the actual check, since we currently use that to
        // avoid negating/reordering, plus the instructions needed to access any
        // stack variables.
        2.into()
    }

    fn generate(
        &self,
        _ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        instruction_count: &mut Address,
    ) -> Result<()> {
        let end = *self
            .end
            .context("Internal error: Forward refeerence")?
            .as_ref();
        if let Some(annotated) = annotated {
            annotated.push(format!("// If: {} @{}", &self.condition, output.len()));
        }
        output.push(format!(
            "jump {} {}",
            // 1 for this instruction not yet added, 1 to skip the next jump.
            *instruction_count.as_ref() + 2,
            self.condition,
        ));
        output.push(format!("jump {} always x false", end));

        Ok(())
    }
}

/// The "else" in an if statement. See `IfOp` for more.
///
/// Preserves: All if no stack vars are used in the condition, otherwise None.
#[derive(Clone, Debug)]
pub struct ElseOp {
    // The first address after the end of the "else" "block".
    pub end: Option<Address>,
}

impl ElseOp {
    pub fn declare() -> ElseOp {
        ElseOp { end: None }
    }
}

impl Operation for ElseOp {
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
        let end = self.end.context("Internal error: Forward refeerence")?;

        if let Some(annotated) = annotated {
            annotated.push(format!("// Else: {} @{}", end, output.len()));
        }

        output.push(format!("jump {} always x false", end));

        Ok(())
    }
}
