use crate::*;

// FIXME: Consider restricting the type to GetStack, SetStack, LoopEnd since that's
// all we use and allowing it more generally could be dangerous.
// Alternatively, create a "StackJump" struct that is more restrictive.
/// As a rule, where used this will represent location-independent code without
/// forward references.
#[derive(Debug, Default, Clone)]
pub struct IrSequence(pub Vec<IrOp>);

impl From<(IrOp, IrOp)> for IrSequence {
    fn from(other: (IrOp, IrOp)) -> IrSequence {
        let mut seq = IrSequence::default();
        seq.0.push(other.0);
        seq.0.push(other.1);
        seq
    }
}

impl From<IrOp> for IrSequence {
    fn from(other: IrOp) -> IrSequence {
        let mut seq = IrSequence::default();
        seq.0.push(other);
        seq
    }
}

impl From<Option<IrOp>> for IrSequence {
    fn from(other: Option<IrOp>) -> IrSequence {
        match other {
            Some(op) => op.into(),
            None => IrSequence::default(),
        }
    }
}

impl IrSequence {
    pub fn push(&mut self, op: IrOp) {
        self.0.push(op)
    }

    pub fn code_size(&self, backend: Backend) -> AddressDelta {
        self.0.iter().map(|op| op.code_size(backend)).sum()
    }
}

/// Many of these are arguably redundant, and/or could be implemented in terms
/// of each other. This is to an extent deliberate, to improve the readability
/// of the annotated output.
///
/// Largely though it's because I'm being lazy and getting away with using the
/// IR as an AST. It works because this language is simple, but would likely
/// fall apart if I wanted to do more optimization, adopt a more recursive
/// structure, etc.
///
/// That's more than I want to do for a weekend project though.
#[derive(Debug, Clone)]
pub enum IrOp {
    CallProc(CallProcOp),
    Label(LabelOp),
    RetProc(RetProcOp),
    Push(PushOp),
    Pop(PopOp),
    Peek(PeekOp),
    Poke(PokeOp),
    Jump(JumpOp),
    MindustryCommand(MindustryOp),
    If(IfOp),
    Else(ElseOp),
    While(WhileOp),
    DoWhile(DoWhileOp),
    InfiniteLoop(InfiniteLoopOp),
    Break(BreakOp),
    Continue(ContinueOp),
    LoopEnd(LoopEndOp),
    Let(LetOp),
    GetStack(GetStackOp),
    SetStack(SetStackOp),
    Set(SetOp),
    Math(MathOp),
    Function(FunctionName, AddressDelta),
    Call(CallOp),
    Return(ReturnOp),
}

pub trait Operation {
    /// Returns the number of instructions for the code generated for this op.
    fn code_size(&self, backend: Backend) -> AddressDelta;

    /// Generates the format used by Mindustry.
    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        instruction_count: &mut Address,
    ) -> Result<()>;
}

impl Operation for IrOp {
    fn code_size(&self, backend: Backend) -> AddressDelta {
        match self {
            IrOp::CallProc(op) => op.code_size(backend),
            IrOp::Push(op) => op.code_size(backend),
            IrOp::Pop(op) => op.code_size(backend),
            IrOp::Peek(op) => op.code_size(backend),
            IrOp::Poke(op) => op.code_size(backend),
            IrOp::GetStack(op) => op.code_size(backend),
            IrOp::SetStack(op) => op.code_size(backend),
            IrOp::Set(op) => op.code_size(backend),
            IrOp::Math(op) => op.code_size(backend),
            IrOp::RetProc(op) => op.code_size(backend),
            IrOp::Label(op) => op.code_size(backend),
            IrOp::MindustryCommand(op) => op.code_size(backend),
            IrOp::Jump(op) => op.code_size(backend),
            IrOp::If(op) => op.code_size(backend),
            IrOp::Else(op) => op.code_size(backend),
            IrOp::While(op) => op.code_size(backend),
            IrOp::DoWhile(op) => op.code_size(backend),
            IrOp::InfiniteLoop(op) => op.code_size(backend),
            IrOp::LoopEnd(op) => op.code_size(backend),
            IrOp::Break(op) => op.code_size(backend),
            IrOp::Continue(op) => op.code_size(backend),
            IrOp::Function(_name, size) => *size,
            IrOp::Return(op) => op.code_size(backend),
            IrOp::Call(op) => op.code_size(backend),
            IrOp::Let(op) => op.code_size(backend),
        }
    }

    fn generate(
        &self,
        ir: &IntermediateRepresentation,
        output: &mut Vec<String>,
        annotated: Option<&mut Vec<String>>,
        instruction_count: &mut Address,
    ) -> Result<()> {
        match self {
            IrOp::CallProc(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Push(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Pop(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Peek(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Poke(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::GetStack(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::SetStack(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Set(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Math(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::RetProc(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Label(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::MindustryCommand(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Jump(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::If(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Else(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::While(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::DoWhile(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::InfiniteLoop(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::LoopEnd(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Break(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Continue(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Function(name, _size) => {
                ir.functions()[name].generate(ir, output, annotated, instruction_count)
            }
            IrOp::Return(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Call(op) => op.generate(ir, output, annotated, instruction_count),
            IrOp::Let(op) => op.generate(ir, output, annotated, instruction_count),
        }
    }
}
