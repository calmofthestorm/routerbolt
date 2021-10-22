use crate::*;

/// Construct generated at the closing `}` of the loop that is common to all
/// loop types.
///
/// Destroys: All
#[derive(Clone, Debug)]
pub struct LoopEndOp {
    // Start of the loop body.
    body_start: Address,

    // Loop condition.
    condition: Condition,
}

trait LoopTrait {
    fn end_address(&self) -> Result<Address>;
    fn condition_address(&self) -> Result<Address>;
}

impl LoopEndOp {
    const SIZE: AddressDelta = AddressDelta::new(1);
}

impl Operation for LoopEndOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        Self::SIZE
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
                "// <loop if>: {} {} @{}",
                &self.condition,
                self.body_start,
                output.len()
            ));
        }

        output.push(format!("jump {} {}", self.body_start, &self.condition));

        Ok(())
    }
}

/// Begins a while loop. The condition is the same as Mindustry's jump. In
/// particular, only one condition may be checked.
///
/// At present this desugars to `While` ... `LoopEnd`, where the While just jumps
/// to the LoopEnd. That's an extra instruction run per loop, but shares more
/// code with the other loops.
///
/// E.g.:
///
/// while lessThan a 7 {
///   op add a a 1
///   print "hello"
/// }
#[derive(Clone, Debug)]
pub struct WhileOp {
    // Start of the loop body.
    body_start: Address,

    // IR instructions that implement the condition check at the end of the
    // loop. Must be position independent.
    end_sequence: Box<IrSequence>,

    // Loop condition.
    condition: Condition,

    // Address where we check the loop condition and then loop or end as
    // appropriate.
    forward: Option<(Address, Address)>,
}

impl WhileOp {
    const SIZE: AddressDelta = AddressDelta::new(1);

    pub fn new(address: Address, end_sequence: IrSequence, condition: Condition) -> WhileOp {
        WhileOp {
            body_start: address + Self::SIZE,
            end_sequence: Box::new(end_sequence),
            forward: None,
            condition,
        }
    }

    pub fn resolve_forward(&mut self, body_end: Address, backend: Backend) -> &IrSequence {
        self.end_sequence.push(IrOp::LoopEnd(LoopEndOp {
            body_start: self.body_start,
            condition: self.condition.clone(),
        }));
        let cond_end = body_end + self.end_sequence.code_size(backend);
        let set = self.forward.replace((body_end, cond_end));
        assert!(set.is_none());
        &self.end_sequence
    }
}

impl LoopTrait for WhileOp {
    fn end_address(&self) -> Result<Address> {
        Ok(self
            .forward
            .context("Internal error: Forward refeerence")?
            .1)
    }

    fn condition_address(&self) -> Result<Address> {
        Ok(self
            .forward
            .context("Internal error: Forward refeerence")?
            .0)
    }
}

impl Operation for WhileOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        // The end sequence is not considered part of the While loop.
        Self::SIZE
    }

    fn generate(
        &self,
        _ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        // Remember, the WhileOp is just the start of the loop. All we do is
        // jump to the condition check. (Workaround to negation, same as with
        // if).
        if let Some(annotated) = annotated {
            annotated.push(format!("// While @{}", output.len()));
        }

        // FIXME: Can optimize by negating. Like if but not as bad. This is
        // jumping to the condition check at the end so that if it fails, we'll
        // end the loop. As an indirect way of negating.
        output.push(format!("jump {} always x false", self.condition_address()?));

        Ok(())
    }
}

/// Begins a do-while loop. The condition is the same as Mindustry's jump. In
/// particular, only one condition may be checked.
///
/// This works by adding a LoopEnd at the end of the body, and is actually more
/// efficient than While as currently implemented.
///
/// E.g.:
///
/// do {
///   print "hello"
///   op add a a 1
/// } while lessThan a 7
#[derive(Clone, Debug)]
pub struct DoWhileOp {
    // Start of the loop body.
    body_start: Address,

    // (condition check start address, condition check end address)
    forward: Option<(Address, Address)>,
}

impl DoWhileOp {
    const SIZE: AddressDelta = AddressDelta::new(0);

    pub fn new(address: Address) -> DoWhileOp {
        DoWhileOp {
            body_start: address + Self::SIZE,
            forward: None,
        }
    }

    pub fn resolve_forward(
        &mut self,
        body_end: Address,
        mut end_sequence: IrSequence,
        condition: Condition,
        backend: Backend,
    ) -> IrSequence {
        end_sequence.push(IrOp::LoopEnd(LoopEndOp {
            body_start: self.body_start,
            condition,
        }));

        let end = body_end + end_sequence.code_size(backend);
        let set = self.forward.replace((body_end, end));
        assert!(set.is_none());

        end_sequence
    }
}

impl LoopTrait for DoWhileOp {
    fn end_address(&self) -> Result<Address> {
        Ok(self
            .forward
            .context("Internal error: Forward refeerence")?
            .1)
    }

    fn condition_address(&self) -> Result<Address> {
        Ok(self
            .forward
            .context("Internal error: Forward refeerence")?
            .0)
    }
}

impl Operation for DoWhileOp {
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
            annotated.push(format!("// Do-While Loop @{}", output.len()));
        }

        Ok(())
    }
}

/// Begins an infinite loop.
///
/// This generates the same code as a do-while loop with "always" condition, but
/// is more efficient than a while loop. Arguably redundant with do-while, but
/// *shrug* it was easy, the lack of && and || will make it more useful, and
/// I've grown fond of the construct in Rust.
///
/// E.g.:
///
/// loop {
///   print "hello"
///   op add a a 1
/// }
#[derive(Clone, Debug)]
pub struct InfiniteLoopOp {
    // Start of the loop body.
    body_start: Address,

    // First instruction after the end of the loop.
    end: Option<Address>,
}

impl InfiniteLoopOp {
    const SIZE: AddressDelta = AddressDelta::new(0);

    pub fn new(address: Address) -> InfiniteLoopOp {
        InfiniteLoopOp {
            body_start: address + Self::SIZE,
            end: None,
        }
    }

    pub fn resolve_forward(&mut self, address: Address) -> IrSequence {
        let op = LoopEndOp {
            condition: Condition::always(),
            body_start: self.body_start,
        };

        let set = self.end.replace(address + LoopEndOp::SIZE);
        assert!(set.is_none());

        IrOp::LoopEnd(op).into()
    }
}

impl LoopTrait for InfiniteLoopOp {
    fn end_address(&self) -> Result<Address> {
        Ok(self.end.context("Internal error: Forward refeerence")?)
    }

    fn condition_address(&self) -> Result<Address> {
        Ok(self.body_start)
    }
}

impl Operation for InfiniteLoopOp {
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
            annotated.push(format!("// InfiniteLoop @{}", output.len()));
        }

        Ok(())
    }
}

/// Breaks out of the top-most enclosing loop. Compile error to use outside a
/// loop.
///
/// Since the only scope is function-level, this is as simple as jumping out.
///
/// FIXME: Support conditions.
#[derive(Clone, Debug)]
pub struct BreakOp {
    /// The index in `ops` of the loop this is in. This lets us avoid a forward
    /// reference here by referencing the loop.
    pub index: IrIndex,
}

impl BreakOp {
    const SIZE: AddressDelta = AddressDelta::new(1);
}

impl Operation for BreakOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        Self::SIZE
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        let end = match &ir.ops()[*self.index] {
            IrOp::While(op) => op.end_address()?,
            IrOp::InfiniteLoop(op) => op.end_address()?,
            IrOp::DoWhile(op) => op.end_address()?,
            // Should have been caught at parse time if input was malformed, so
            // this is a bug.
            _ => unreachable!("Break not from recognized loop"),
        };

        if let Some(annotated) = annotated {
            annotated.push(format!("// Break @{}", output.len()));
        }

        output.push(format!("jump {} always x false", end));

        Ok(())
    }
}

/// Skips the remainder of this iteration of the top-most enclosing loop,
/// returning to the start. Compile error to use outside a loop.
///
/// Since the only scope is function-level, this is as simple as jumping to the
/// condition at the end. I have verified that C++ checks the condition of a
/// do-while after continue before returning to the start of the loop, and we
/// follow that here.
///
/// FIXME: Support conditions.
#[derive(Clone, Debug)]
pub struct ContinueOp {
    /// The index in `ops` of the loop this is in. This lets us avoid a forward
    /// reference here by referencing the loop.
    pub index: IrIndex,
}

impl ContinueOp {
    const SIZE: AddressDelta = AddressDelta::new(1);
}

impl Operation for ContinueOp {
    fn code_size(&self, _backend: Backend) -> AddressDelta {
        Self::SIZE
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        _instruction_count: &mut Address,
    ) -> Result<()> {
        let condition_check = match &ir.ops()[*self.index] {
            IrOp::While(op) => op.condition_address()?,
            IrOp::InfiniteLoop(op) => op.condition_address()?,
            IrOp::DoWhile(op) => op.condition_address()?,
            // Should have been caught at parse time if input was malformed, so
            // this is a bug.
            _ => unreachable!("Break not from recognized loop"),
        };

        if let Some(annotated) = annotated {
            annotated.push(format!("// Continue @{}", output.len()));
        }

        output.push(format!("jump {} always x false", condition_check));

        Ok(())
    }
}
