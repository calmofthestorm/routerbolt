use std::collections::HashMap;
use std::convert::TryInto;
use std::rc::Rc;

use anyhow::bail;

use crate::*;

pub fn parse(text: &str) -> Result<IntermediateRepresentation> {
    let mut context = ParserContext {
        ops: Vec::default(),
        // FIXME: Refactor this is bad.
        backend: Backend::Internal, // temporary until preprocess over
        instruction_count: Address::from(0),
        scope_stack: Vec::default(),
        functions: HashMap::default(),
        labels: HashMap::default(),
        has_stack: false,
    };

    let mut stack_config = None;

    let mut preparse_fn_stack = Vec::default();
    for (line_no, line) in text.lines().enumerate() {
        context
            .preparse_line(
                &lex_line(clean_line(line)),
                &mut stack_config,
                &mut preparse_fn_stack,
            )
            .with_context(|| format!("Preparse Line {}: {}", line_no, line))?;
    }

    let stack_config = stack_config.unwrap_or(StackConfig::Internal(0));

    // We may need to zero the stack pointer if using one.
    let (has_stack, backend) = match &stack_config {
        StackConfig::Internal(size) if *size == 0 => (false, Backend::Internal),
        StackConfig::Internal(..) => (true, Backend::Internal),
        StackConfig::External(..) => (true, Backend::External),
    };

    context.backend = backend;

    context.has_stack = has_stack;
    if has_stack {
        let op = SetOp::new(MindustryTerm::stack_sz(), MindustryTerm::zero());
        context.instruction_count += op.code_size(backend);
        context.ops.push(IrOp::Set(op));
    }

    for (line_no, line) in text.lines().enumerate() {
        // Some ops update this state themselves, but we pull out the common case of one op here.
        let clean = clean_line(line);
        for op in context
            .parse_line(clean, &lex_line(clean_line(line)))
            .with_context(|| format!("Line {}: {}", line_no, line))?
            .0
        {
            context.instruction_count += op.code_size(context.backend);
            context.ops.push(op);
        }
    }

    let backend_params = match &stack_config {
        StackConfig::Internal(stack_size) => {
            let push_entry_size = 3;
            let pop_entry_size = 2;
            let poke_entry_size = 2;
            let push_table_start = context.instruction_count + 1.into();
            let pop_table_start =
                push_table_start + AddressDelta::from(push_entry_size * stack_size);
            let poke_table_start =
                pop_table_start + AddressDelta::from(pop_entry_size * stack_size);

            let int = InternalParams {
                push_entry_size: push_entry_size.into(),
                pop_entry_size: pop_entry_size.into(),
                poke_entry_size: poke_entry_size.into(),
                push_table_start,
                pop_table_start,
                poke_table_start,
            };

            BackendParams::Internal(Rc::new(int))
        }
        StackConfig::External(cell_name) => {
            let ext = ExternalParams {
                cell_name: cell_name.clone(),
            };
            BackendParams::External(Rc::new(ext))
        }
    };

    Ok(IntermediateRepresentation {
        ops: context.ops,
        stack_config,
        functions: context
            .functions
            .into_iter()
            .map(|(k, v)| (k, Rc::new(v)))
            .collect(),
        labels: context.labels,
        backend,
        backend_params,
    })
}

struct ParserContext {
    // The IR instructions being emitted.
    ops: Vec<IrOp>,

    // The number of output instructions that will be emitted by the
    // ops we have thus far. Each IrOp is typically a fixed number
    // of Mindustry statements (usually more than one), but a few
    // (such as function calls) vary.
    instruction_count: Address,

    // Backend being used (internal stack based on jump table or
    // external memory cell).
    backend: Backend,

    // Tracks structures that desugar into multiple statements (if/else, loops,
    // functions, etc). Note that in the look up sense, only functions are
    // scopes. A variable access inside a loop has the same rules as anywhere
    // else in the function body/global scope it rolls up to.
    //
    // These are indices into `ops`.
    scope_stack: Vec<IrIndex>,

    // Function definitions.
    functions: HashMap<FunctionName, FunctionOp>,

    // Jump labels.
    labels: HashMap<LabelName, Address>,

    // FIXME: Refactor this, backend, et al and init order.
    has_stack: bool,
}

impl ParserContext {
    // FIXME: Try to share more code between preparse and parse. It's
    // straightforward to share more parsing code; sharing the state logic is
    // harder because things will be an error in psas2 that are expected in
    // pass1.
    /// Specialized initial pass to parse function definitions and stack size
    /// setting, so that they can be used before they are defined, since we
    /// need the definition to determine the call site size.
    fn preparse_line(
        &mut self,
        tok: &[&str],
        stack_config: &mut Option<StackConfig>,
        preparse_fn_stack: &mut Vec<Option<FunctionName>>,
    ) -> Result<()> {
        match tok.get(0).copied() {
            Some("fn") => self.preparse_function(&tok[1..], preparse_fn_stack),
            Some("let") => self.preparse_let(&tok[1..], preparse_fn_stack),
            Some("stack_config") => self.preparse_stack_config(&tok[1..], stack_config),
            Some("}") if tok.last().copied() == Some("{") => Ok(()),
            Some("}") => {
                preparse_fn_stack.pop().context("missing opening {")?;
                Ok(())
            }
            _ => {
                if let Some("{") = tok.last().copied() {
                    preparse_fn_stack.push(None);
                }

                Ok(())
            }
        }
    }

    fn preparse_stack_config(
        &mut self,
        tok: &[&str],
        stack_config: &mut Option<StackConfig>,
    ) -> Result<()> {
        if tok.len() != 2 || (tok[0] != "size" && tok[0] != "cell") {
            bail!("form is `stack_config [ size <stack_size> | cell <cell_name> ]` {");
        }

        if stack_config.is_some() {
            bail!("stack config set for second time here");
        }

        if tok[0] == "size" {
            let size: usize = tok[1]
                .parse()
                .context("stack size must be a non-negative integer")?;
            stack_config.replace(StackConfig::Internal(size));
        } else {
            stack_config.replace(StackConfig::External(Rc::new(tok[1].to_string())));
        }

        Ok(())
    }

    fn preparse_function(
        &mut self,
        tok: &[&str],
        preparse_fn_stack: &mut Vec<Option<FunctionName>>,
    ) -> Result<()> {
        if tok.len() < 2 || *tok.last().unwrap() != "{" {
            bail!("form is `fn name [arg1 [arg2...]] [-> [return1 [return2...]]]` {");
        }

        let name: FunctionName = tok[0].try_into().context("function name")?;
        let (args, returns) = parse_arrow(&tok[1..tok.len() - 1])?;
        let func = FunctionOp::declare(name.clone(), args, returns)?;
        preparse_fn_stack.push(Some(name.clone()));
        if self.functions.insert(name.clone(), func).is_some() {
            bail!("function {} is defined a second time here", name);
        }
        Ok(())
    }

    fn preparse_let(
        &mut self,
        tok: &[&str],
        preparse_fn_stack: &mut Vec<Option<FunctionName>>,
    ) -> Result<()> {
        if tok.len() != 1 {
            bail!("form is `let *stack_var_name`");
        }

        let name = tok[0];

        let mut it = preparse_fn_stack.iter().rev();
        let function_name = loop {
            match it.next() {
                None => bail!("let may only be used within a function",),
                Some(None) => {}
                Some(Some(f)) => break f,
            }
        };

        let name: StackVar = name.try_into().with_context(|| {
            format!(
                "let binding \"{}\" is not a stack var (does not start with '*')",
                name
            )
        })?;

        let function = self.functions.get_mut(function_name).unwrap();

        let pos = FrameIndex::from(function.locals.len());
        if function.locals.insert(name.clone(), pos).is_some() {
            bail!("{} is defined a second time here", &name);
        }

        Ok(())
    }

    fn require_stack(&self) -> Result<()> {
        if !self.has_stack {
            bail!("This function requires that a stack be configured. Use, e.g., `stack_config cell bank1` to use an external memory bank or `stack_config size <size>` for an internal jump-table stack. Size must be greater than 0, since setting it to 0 explicitly disables the stack.");
        } else {
            Ok(())
        }
    }

    fn parse_line(&mut self, line: &str, tok: &[&str]) -> Result<IrSequence> {
        if tok.is_empty() {
            return Ok(None.into());
        }

        if tok[0] == "stack_config" {
            // Handled in first pass.
            Ok(None.into())
        } else if tok[0] == "callproc" {
            self.parse_callproc(&tok[1..])
        } else if tok[0] == "ret" {
            self.parse_ret(&tok[1..])
        } else if tok[0].ends_with(":") && tok.len() == 1 {
            let name = &tok[0][..tok[0].len() - 1];
            self.parse_label(name)
        } else if tok[0].starts_with("//") {
            // Comment
            Ok(None.into())
        } else if tok[0] == "push" {
            self.parse_push(&tok[1..])
        } else if tok[0] == "poke" {
            self.parse_poke(&tok[1..])
        } else if tok[0] == "peek" {
            self.parse_peek(&tok[1..])
        } else if tok[0] == "pop" {
            self.parse_pop(&tok[1..])
        } else if tok[0] == "jump" {
            self.parse_jump(&tok[1..])
        } else if tok[0] == "do" {
            self.parse_do(&tok[1..])
        } else if tok[0] == "while" {
            self.parse_while(&tok[1..])
        } else if tok[0] == "loop" {
            self.parse_loop(&tok[1..])
        } else if tok[0] == "break" {
            self.parse_break(&tok[1..])
        } else if tok[0] == "continue" {
            self.parse_continue(&tok[1..])
        } else if tok[0] == "if" {
            self.parse_if(&tok[1..])
        } else if tok[0] == "fn" {
            self.parse_function(&tok[1..])
        } else if tok[0] == "return" {
            self.parse_return(&tok[1..])
        } else if tok[0] == "call" {
            self.parse_call(&tok[1..])
        } else if tok[0] == "let" {
            self.parse_let(&tok[1..])
        } else if tok[0] == "}" {
            self.parse_closing_brace(&tok[1..])
        } else if tok[0] == "op" {
            self.parse_op(&tok[1..])
        } else if tok[0] == "set" {
            self.parse_set(line)
        } else if tok[0] == "print" {
            self.parse_print(line)
        } else {
            self.parse_mindustry_command(&tok)
        }
    }

    fn parse_callproc(&mut self, tok: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        if tok.len() != 1 {
            bail!("form is `callproc label`");
        }
        let target = tok[0].try_into().context("callproc target label")?;
        Ok(IrOp::CallProc(CallProcOp { target }).into())
    }

    fn parse_ret(&mut self, tok: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        if !tok.is_empty() {
            bail!("form is `ret`");
        }

        Ok(IrOp::RetProc(RetProcOp {}).into())
    }

    fn parse_label(&mut self, name: &str) -> Result<IrSequence> {
        let target: LabelName = name.try_into().context("label statement label")?;
        let prev = self.labels.insert(target.clone(), self.instruction_count);
        if prev.is_some() {
            bail!("label {} is defined a second time here", target);
        }
        Ok(IrOp::Label(LabelOp { target }).into())
    }

    fn parse_push(&mut self, tok: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        if !tok.is_empty() {
            bail!("form is `push`");
        }

        Ok(IrOp::Push(PushOp {}).into())
    }

    fn parse_pop(&mut self, tok: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        if !tok.is_empty() {
            bail!("form is `pop`");
        }

        Ok(IrOp::Pop(PopOp {}).into())
    }

    fn parse_peek(&mut self, tok: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        let depth = if tok.len() == 0 {
            MindustryTerm::zero()
        } else if tok.len() == 1 {
            tok[0].try_into().context("peek depth")?
        } else {
            bail!("form is `peek [depth]`")
        };

        Ok(IrOp::Peek(PeekOp { depth }).into())
    }

    fn parse_poke(&mut self, tok: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        let depth = if tok.len() == 0 {
            MindustryTerm::zero()
        } else if tok.len() == 1 {
            tok[0].try_into().context("poke depth")?
        } else {
            bail!("form is `poke [depth]`");
        };

        Ok(IrOp::Poke(PokeOp { depth }).into())
    }

    fn parse_jump(&mut self, tok: &[&str]) -> Result<IrSequence> {
        if tok.len() < 2 {
            bail!("form is `jump label condition`")
        }

        let cond = self.parse_condition(&tok[1..]);
        let (mut ir_seq, condition) = cond.context("jump condition")?;

        let target = tok[0].try_into().context("jump label")?;
        ir_seq.push(IrOp::Jump(JumpOp { target, condition }).into());
        Ok(ir_seq)
    }

    fn parse_while(&mut self, tok: &[&str]) -> Result<IrSequence> {
        if tok.last().copied() != Some("{") {
            bail!("form is `while condition {`")
        }

        // Generate the sequence of instructions that will go at the END of the
        // loop.
        let cond = self.parse_condition(&tok[..tok.len() - 1]);
        let (end_seq, condition) = cond.context("while condition")?;
        let op = WhileOp::new(self.instruction_count, end_seq, condition);

        // This function only adds to ops the instructions to start the loop. We
        // generate the end, but then save it for when we get there.
        self.scope_stack.push(self.ops.len().into());

        Ok(IrOp::While(op).into())
    }

    fn parse_do(&mut self, tok: &[&str]) -> Result<IrSequence> {
        if tok.len() != 1 || tok[0] != "{" {
            bail!("form is `do {`");
        }

        self.scope_stack.push(self.ops.len().into());

        Ok(IrOp::DoWhile(DoWhileOp::new(self.instruction_count)).into())
    }

    fn parse_loop(&mut self, tok: &[&str]) -> Result<IrSequence> {
        if tok.len() != 1 || tok[0] != "{" {
            bail!("form is `loop {`");
        }

        self.scope_stack.push(self.ops.len().into());

        Ok(IrOp::InfiniteLoop(InfiniteLoopOp::new(self.instruction_count)).into())
    }

    fn parse_break(&mut self, tok: &[&str]) -> Result<IrSequence> {
        if !tok.is_empty() {
            bail!("form is `break`");
        }

        let index = self
            .find_enclosing_loop_index()?
            .context("break not valid outside loop")?;

        Ok(IrOp::Break(BreakOp { index }).into())
    }

    fn parse_continue(&mut self, tok: &[&str]) -> Result<IrSequence> {
        if !tok.is_empty() {
            bail!("form is `continue`");
        }

        let index = self
            .find_enclosing_loop_index()?
            .context("continue not valid outside loop")?;

        Ok(IrOp::Continue(ContinueOp { index }).into())
    }

    fn parse_if(&mut self, tok: &[&str]) -> Result<IrSequence> {
        if tok.last().copied() != Some("{") {
            bail!("form is `if condition {`")
        }

        let cond = self.parse_condition(&tok[..tok.len() - 1]);
        let (mut ir_sequence, condition) = cond.context("if condition")?;

        self.scope_stack
            .push((ir_sequence.0.len() + self.ops.len()).into());

        ir_sequence.push(IrOp::If(IfOp::new(condition)));
        Ok(ir_sequence)
    }

    fn parse_function(&mut self, tok: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        // We already validated the form in pre-processing.
        let name: FunctionName = tok[0].try_into().unwrap();
        let function = self.functions.get_mut(&name).unwrap();
        function.start_parse(self.instruction_count);

        self.scope_stack.push(self.ops.len().into());

        Ok(IrOp::Function(name, function.code_size(self.backend)).into())
    }

    fn parse_return(&mut self, value_names: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        let function_name = self
            .find_enclosing_function()?
            .context("return may not be used outside a function")?;
        let function = &self.functions[&function_name];
        let statement = ReturnOp::new(function, value_names, self.backend);
        statement
            .with_context(|| {
                format!(
                    "from function {} with values \"{:?}\"",
                    &function_name, value_names,
                )
            })
            .map(IrOp::Return)
            .map(Into::into)
    }

    /// If any of the args or return values are stack variables, this call
    /// site must be in a function, and the binding must exist in its frame.
    fn parse_call_variable(
        &self,
        name: &str,
        function_name: &Option<FunctionName>,
    ) -> Result<Term> {
        self.require_stack()?;
        // `in_function` is the function the *call site* is in, not the function
        // being called.
        let arg: Term = name.try_into()?;
        match (function_name.as_ref(), &arg) {
            (Some(function_name), Term::StackVar(stack_arg)) => {
                let function = &self.functions[&function_name];
                let local = function.locals.get(&stack_arg);
                local
                    .with_context(|| {
                        format!(
                            "function {} does not have stack variable {}",
                            &function_name, &stack_arg
                        )
                    })
                    .map(|_| arg)
            }
            (None, Term::StackVar(arg)) => {
                bail!(
                    "{} is a stack variable and may only be used inside a function",
                    &arg
                );
            }
            _ => Ok(arg),
        }
    }

    fn parse_call(&mut self, tok: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        if tok.len() < 1 {
            bail!("form is `call name [args] [-> return_values]");
        }

        let name = tok[0].try_into().context("function name")?;

        let (arg_names, return_names) = parse_arrow(&tok[1..])?;

        let call_site_function = self.find_enclosing_function()?;

        let mut args = Vec::with_capacity(arg_names.len());
        for (j, arg) in arg_names.iter().copied().enumerate() {
            let arg = self
                .parse_call_variable(arg, &call_site_function)
                .with_context(|| format!("parameter {} \"{}\"", j, arg))?;
            args.push(arg.into());
        }
        let mut returns = Vec::with_capacity(return_names.len());
        for (j, ret) in return_names.iter().copied().enumerate() {
            let ret = self
                .parse_call_variable(ret, &call_site_function)
                .with_context(|| format!("return binding {} \"{}\"", j, ret))?;
            let ret = ret.into();
            if returns.contains(&ret) {
                bail!("return binding {} \"{}\" is duplicated", j, ret)
            }
            returns.push(ret);
        }

        let function = self
            .functions
            .get(&name)
            .with_context(|| format!("function definition for {} not found", &name))?;

        if function.args.len() != args.len() {
            bail!(
                "function {} takes {} args but called with {} values",
                &name,
                function.args.len(),
                args.len()
            );
        }

        if function.returns.len() != returns.len() {
            bail!(
                "function {} returns {} values but being bound to {} bindings",
                &name,
                function.returns.len(),
                returns.len()
            );
        }

        Ok(IrOp::Call(CallOp::new(
            args,
            returns,
            function.locals.len(),
            name.clone(),
            call_site_function,
            self.backend,
        ))
        .into())
    }

    fn parse_let(&mut self, tok: &[&str]) -> Result<IrSequence> {
        self.require_stack()?;
        // FIXME: Restrict that let must preceed use.

        // No actual work to do -- was preprocessed -- but want to annotate.
        let name = tok[0];
        let function_name = self
            .find_enclosing_function()?
            .context("let may not be used outside a function")?;
        let function = &self.functions[&function_name];
        let name: StackVar = name.try_into().unwrap();
        let pos = FrameIndex::from(function.locals.len());
        Ok(IrOp::Let(LetOp { name, pos }).into())
    }

    fn parse_op(&mut self, tok: &[&str]) -> Result<IrSequence> {
        let operation = Rc::new(tok[0].to_string());
        let dest: Term = tok[1].try_into().context("op dest")?;
        let arg1: Term = tok[2].try_into().context("op arg1")?;
        let arg2: Term = tok[3].try_into().context("op arg2")?;
        let function = self.find_enclosing_function()?;
        let (mut seq, dest, arg1, arg2, mut write) =
            ir_read_two_write_one(dest, arg1, arg2, &function)?;
        seq.push(IrOp::Math(MathOp {
            operation,
            dest,
            arg1,
            arg2,
        }));
        seq.0.append(&mut write.0);
        Ok(seq)
    }

    fn parse_print(&mut self, line: &str) -> Result<IrSequence> {
        let value: Term = line.trim()[5..].trim().try_into().context("print value")?;
        let (mut seq, value) = ir_read_one_arg(value, &self.find_enclosing_function()?)?;
        seq.push(IrOp::MindustryCommand(MindustryOp {
            command: vec![Rc::new(format!("print {}", &value))]
                .try_into()
                .context("create print command")?,
        }));
        Ok(seq)
    }

    fn parse_set(&mut self, line: &str) -> Result<IrSequence> {
        if let Some((dest, source)) = line.trim()["set".len()..]
            .trim()
            .split_once(|c: char| c.is_whitespace())
        {
            let dest: Term = dest.try_into().context("set dest")?;
            let source: Term = source.try_into().context("set source")?;
            ir_copy_arg(dest, source, &self.find_enclosing_function()?)
        } else {
            bail!("set form is `set a b`");
        }
    }

    fn parse_closing_brace(&mut self, tok: &[&str]) -> Result<IrSequence> {
        let open_index = match self.scope_stack.pop() {
            Some(index) => index,
            None => {
                bail!("scope stack is empty");
            }
        };

        if tok.len() == 0 {
            self.handle_single_closing_brace(open_index)
        } else {
            self.handle_closing_brace_more(tok, open_index)
        }
    }

    fn parse_mindustry_command(&mut self, tok: &[&str]) -> Result<IrSequence> {
        let command = tok.iter().copied().map(String::from).map(Rc::new);
        let command: Vec<Rc<String>> = command.collect();
        let command = command.try_into().context("parse mindustry command")?;
        let command = MindustryOp { command: command };
        Ok(IrOp::MindustryCommand(command).into())
    }

    /// If the condition uses stack vars, get them and adjust the condition
    /// to use the temporaries.
    fn parse_condition(&self, tok: &[&str]) -> Result<(IrSequence, Condition)> {
        parse_condition(self.find_enclosing_function()?, tok)
    }

    /// Finds the top-most enclosing function definition, skipping over ifs and
    /// loops.
    fn find_enclosing_function(&self) -> Result<Option<FunctionName>> {
        Self::find_enclosing_function_internal(&self.scope_stack, &self.ops)
    }

    fn find_enclosing_function_internal(
        scope_stack: &[IrIndex],
        ops: &[IrOp],
    ) -> Result<Option<FunctionName>> {
        for index in scope_stack.iter().rev() {
            let op = &ops[**index];
            match op {
                IrOp::InfiniteLoop(..)
                | IrOp::DoWhile(..)
                | IrOp::While(..)
                | IrOp::If(..)
                | IrOp::Else(..) => {}
                IrOp::Function(f, _) => {
                    return Ok(Some(f.clone()));
                }
                _ => bail!("Internal error: unexpected op {:?} on scope stack", op),
            }
        }

        Ok(None)
    }

    /// Finds the top-most loop index, skipping over ifs. Stops at function
    /// boundaries.
    fn find_enclosing_loop_index(&self) -> Result<Option<IrIndex>> {
        for index in self.scope_stack.iter().rev() {
            let op = &self.ops[**index];
            match op {
                IrOp::InfiniteLoop(..) | IrOp::DoWhile(..) | IrOp::While(..) => {
                    return Ok(Some(*index));
                }
                IrOp::If(..) | IrOp::Else(..) => {}
                IrOp::Function(..) => return Ok(None),
                _ => bail!("Internal error: unexpected op {:?} on scope stack", op),
            }
        }
        Ok(None)
    }

    fn handle_closing_brace_more(
        &mut self,
        tok: &[&str],
        open_index: IrIndex,
    ) -> Result<IrSequence> {
        let enclosing_function =
            Self::find_enclosing_function_internal(&self.scope_stack, &self.ops)?;
        if tok.len() == 2 && tok[0] == "else" && tok[1] == "{" {
            match &mut self.ops[*open_index] {
                IrOp::If(ref mut if_op) => {
                    let op = IrOp::Else(ElseOp::declare());
                    if_op.resolve_forward(self.instruction_count + op.code_size(self.backend));
                    self.scope_stack.push(self.ops.len().into());
                    Ok(op.into())
                }
                _ => bail!("else does not match if statement structurally"),
            }
        } else if tok.len() >= 1 && tok[0] == "while" {
            // DoWhile case. Only needed for break/continue.
            match &mut self.ops[*open_index] {
                IrOp::DoWhile(ref mut do_while_op) => {
                    let cond = parse_condition(enclosing_function, &tok[1..]);
                    let (end_seq, condition) = cond.context("do-while condition")?;
                    let ops = do_while_op.resolve_forward(
                        self.instruction_count,
                        end_seq,
                        condition,
                        self.backend,
                    );
                    Ok(ops)
                }
                _ => bail!("`} while x y z` construct is only valid as part of a do-while loop"),
            }
        } else {
            bail!("unknown form of }}: {:?}", tok);
        }
    }

    fn handle_single_closing_brace(&mut self, open_index: IrIndex) -> Result<IrSequence> {
        let op = &mut self.ops[*open_index];
        match op {
            IrOp::Else(ref mut else_op) => {
                let set = else_op.end.replace(self.instruction_count);
                assert!(set.is_none());
                Ok(None.into())
            }
            IrOp::InfiniteLoop(ref mut loop_op) => {
                Ok(loop_op.resolve_forward(self.instruction_count))
            }
            IrOp::Function(_func, _size) => {
                // FIXME: at present, we don't check that all paths
                // return. That would be hard to do without actually
                // recursively parsing the input. At this time, user
                // is responsible for making all paths return the
                // correct number of arguments, and failing to do so
                // is undefined behavior. This includes return in a void function as
                // well.
                //
                // Therefore, the interesting behavior is in Return.
                Ok(None.into())
            }
            IrOp::If(ref mut if_op) => {
                if_op.resolve_forward(self.instruction_count);
                Ok(None.into())
            }
            IrOp::While(ref mut while_op) => {
                // FIXME: I dislike the clone here because it could lead to an
                // unresolved forward reference if forward references ever snuck
                // into the IrSequence. It would be safer to replace it with a
                // less general type.
                Ok(while_op
                    .resolve_forward(self.instruction_count, self.backend)
                    .clone())
            }
            _ => unreachable!("unexpected op {:?} on scope stack", op),
        }
    }
}

fn parse_condition(
    function: Option<FunctionName>,
    tok: &[&str],
) -> Result<(IrSequence, Condition)> {
    if tok[0] == "always" {
        return Ok((None.into(), Condition::always()));
    } else if tok[0] == "never" {
        return Ok((None.into(), Condition::never()));
    }

    if tok.len() != 3 {
        bail!("condition form is `cond a b`, `always`, or `never`")
    }

    // FIXME: validate the condition?
    let cond = Rc::new(tok[0].to_string());

    let arg1: Term = tok[1].try_into().context("condition arg1")?;
    let arg2: Term = tok[2].try_into().context("condition arg2")?;

    let (read_sequence, arg1, arg2) = ir_read_two_args(arg1, arg2, &function)?;
    let condition = (cond, arg1, arg2).try_into().context("condition")?;

    Ok((read_sequence, condition))
}

/// Takes a token sequence like `foo bar -> qux` and splits on the arrow,
/// ensuring there is at most one arrow. If the arrow is omitted, all tokens are
/// interpreted as preceeding it.
fn parse_arrow<'a, 'b>(tokens: &'a [&'b str]) -> Result<(&'a [&'b str], &'a [&'b str])> {
    let mut it = tokens.split(|tok| *tok == "->");
    match it.next() {
        Some(first) => match it.next() {
            Some(..) if it.next().is_some() => {
                bail!("-> may appear at most once");
            }
            Some(second) => Ok((first, second)),
            None => {
                if first.is_empty() {
                    Ok((&[], &[]))
                } else if first[0] == "->" {
                    Ok((&[], &first[1..]))
                } else {
                    Ok((first, &[]))
                }
            }
        },
        None => Ok((&[], &[])),
    }
}

fn clean_line(line: &str) -> &str {
    let mut line = line.trim();

    // A convenience. It's hard to remember not to add them when writing
    // C-like syntax, and they aren't ambiguous with anything.
    while line.ends_with(";") {
        let l = line.len();
        line = &line[..l - 1];
    }

    line
}

fn lex_line(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arrow_empty() {
        for text in &["", "->", " ->", "-> "] {
            let text: Vec<_> = text.trim().split_whitespace().collect();
            let (a, b) = parse_arrow(&text).unwrap();
            assert!(a.is_empty());
            assert!(b.is_empty());
        }
    }

    #[test]
    fn test_parse_arrow_multi() {
        let text = "Do you recall how it came to that place";
        let text: Vec<_> = text.split_whitespace().collect();
        let (a, b) = parse_arrow(&text).unwrap();
        assert_eq!(a, text.as_slice());
        assert!(b.is_empty());
    }

    #[test]
    fn test_parse_arrow_multi_right() {
        let text = "-> And they sang of their lightnings and shapeful disgrace";
        let text: Vec<_> = text.split_whitespace().collect();
        let (a, b) = parse_arrow(&text).unwrap();
        assert!(a.is_empty());
        assert_eq!(b, &text[1..]);
    }

    #[test]
    fn test_parse_arrow_multi_left() {
        let text = "And It tilted Its vanes and ennobled Its spires ->";
        let text: Vec<_> = text.split_whitespace().collect();
        let (a, b) = parse_arrow(&text).unwrap();
        assert_eq!(a, &text[..text.len() - 1]);
        assert!(b.is_empty());
    }

    #[test]
    fn test_parse_arrow_multi_middle() {
        let text = "They welcomed It then -> and commingled all choirs.";
        let text: Vec<_> = text.split_whitespace().collect();
        let (a, b) = parse_arrow(&text).unwrap();
        assert_eq!(a, &["They", "welcomed", "It", "then"]);
        assert_eq!(b, &["and", "commingled", "all", "choirs."]);
    }

    #[test]
    fn test_parse_arrow_single_left() {
        let text = "And ->";
        let text: Vec<_> = text.split_whitespace().collect();
        let (a, b) = parse_arrow(&text).unwrap();
        assert_eq!(a, &["And"]);
        assert!(b.is_empty());
    }

    #[test]
    fn test_parse_arrow_single_right() {
        let text = "-> not";
        let text: Vec<_> = text.split_whitespace().collect();
        let (a, b) = parse_arrow(&text).unwrap();
        assert!(a.is_empty());
        assert_eq!(b, &["not"]);
    }

    #[test]
    fn test_parse_arrow_error() {
        for text in &[
            "-> ->",
            "-> enough ->",
            "-> -> still",
            "not -> ->",
            "enough -> -> still",
            "-> it mourns ->",
        ] {
            let text: Vec<_> = text.split_whitespace().collect();
            assert!(parse_arrow(&text).is_err());
        }
    }
}
