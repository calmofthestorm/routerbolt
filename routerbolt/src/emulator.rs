use std::collections::HashMap;
use std::rc::Rc;

use crate::*;

/// Simple emulator for a small subset of Mindustry programs. The goal here is
/// to write control flow tests, so we only need a handful of operations. I've
/// taken various shortcuts here (e.g., all values are integers, conditionals
/// just treat anything involving null as false, etc).

#[derive(Clone, Debug)]
pub struct Cell {
    name: Rc<String>,
    data: Vec<Option<usize>>,
}

impl Cell {
    pub fn new(name: Rc<String>) -> Cell {
        Cell {
            data: vec![None; 512],
            name,
        }
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Self::new(Rc::new("bank1".to_string()))
    }
}

pub struct Emulator {
    cell: Option<Cell>,
    instructions: Vec<Instruction>,
    vars: HashMap<Rc<String>, usize>,
    counter: Rc<String>,
    watches: Vec<Rc<String>>,
    breakpoints: Vec<usize>,
    print_buffer: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Math {
    Add,
    Sub,
    Mul,
    Mod,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    Always,
    Lt,
    Gt,
    Eq,
    Ne,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Instruction {
    // As end, except don't reset instruction pointer -- just move past the pause.
    Pause,
    End,
    Math(Math, Rc<String>, Rc<String>, Rc<String>),
    Read(Rc<String>, Rc<String>, Rc<String>),
    Write(Rc<String>, Rc<String>, Rc<String>),
    Set(Rc<String>, Rc<String>),
    Jump(Cond, usize, Rc<String>, Rc<String>),
    Print(Rc<String>),
    PrintFlush(Rc<String>),
}

impl std::fmt::Display for Math {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Math::Add => "add".fmt(f),
            Math::Sub => "sub".fmt(f),
            Math::Mul => "mul".fmt(f),
            Math::Mod => "mod".fmt(f),
        }
    }
}

impl std::fmt::Display for Cond {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Cond::Always => "always".fmt(f),
            Cond::Lt => "lessThan".fmt(f),
            Cond::Gt => "greaterThan".fmt(f),
            Cond::Eq => "equal".fmt(f),
            Cond::Ne => "notEqual".fmt(f),
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Instruction::Pause => "pause".fmt(f),
            Instruction::End => "end".fmt(f),
            Instruction::Math(op, dest, arg1, arg2) => {
                write!(f, "op {} {} {} {}", op, dest, arg1, arg2)
            }
            Instruction::Read(name, cell, address) => {
                write!(f, "read {} {} {}", name, cell, address)
            }
            Instruction::Write(name, cell, address) => {
                write!(f, "write {} {} {}", name, cell, address)
            }
            Instruction::Set(dest, source) => {
                write!(f, "set {} {}", dest, source)
            }
            Instruction::Jump(cond, dest, arg1, arg2) => {
                write!(f, "jump {} {} {} {}", dest, cond, arg1, arg2)
            }
            Instruction::Print(what) => {
                write!(f, "print {}", what)
            }
            Instruction::PrintFlush(output) => {
                write!(f, "printflush {}", output)
            }
        }
    }
}

impl Emulator {
    pub fn new(cell: Option<Cell>, program: &str) -> Result<Emulator> {
        let mut instructions = Vec::default();

        for (line_no, line) in program.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let tok: Vec<_> = line.split_whitespace().collect();

            if tok[0] == "end" {
                check_n_tok(&tok, 1, line_no)?;
                instructions.push(Instruction::End);
            } else if tok[0] == "pause" {
                check_n_tok(&tok, 1, line_no)?;
                instructions.push(Instruction::Pause);
            } else if tok[0] == "op" {
                check_n_tok(&tok, 5, line_no)?;
                let out = Rc::new(tok[2].to_string());
                let arg1 = Rc::new(tok[3].to_string());
                let arg2 = Rc::new(tok[4].to_string());
                let op = if tok[1] == "add" {
                    Math::Add
                } else if tok[1] == "sub" {
                    Math::Sub
                } else if tok[1] == "mul" {
                    Math::Mul
                } else if tok[1] == "mod" {
                    Math::Mod
                } else {
                    bail!(
                        "Line {}: unsupported op command {} (emulator only supports add, mul, sub)",
                        tok[1],
                        line_no
                    );
                };
                instructions.push(Instruction::Math(op, out, arg1, arg2));
            } else if tok[0] == "read" || tok[0] == "write" {
                check_n_tok(&tok, 4, line_no)?;
                let name = Rc::new(tok[1].to_string());
                let cell = Rc::new(tok[2].to_string());
                let address = Rc::new(tok[3].to_string());

                if tok[0] == "read" {
                    instructions.push(Instruction::Read(name, cell, address));
                } else {
                    instructions.push(Instruction::Write(name, cell, address));
                }
            } else if tok[0] == "set" {
                check_n_tok(&tok, 3, line_no)?;
                let dest = Rc::new(tok[1].to_string());
                let source = Rc::new(tok[2].to_string());
                instructions.push(Instruction::Set(dest, source));
            } else if tok[0] == "jump" {
                check_n_tok(&tok, 5, line_no)?;
                let cond = Rc::new(tok[2].to_string());
                let dest: usize = tok[1]
                    .parse()
                    .context("Line {}: jump dest must be integer")?;
                let op1 = Rc::new(tok[3].to_string());
                let op2 = Rc::new(tok[4].to_string());
                let c = if *cond == "equal" {
                    Cond::Eq
                } else if *cond == "notEqual" {
                    Cond::Ne
                } else if *cond == "lessThan" {
                    Cond::Lt
                } else if *cond == "greaterThan" {
                    Cond::Gt
                } else if *cond == "always" {
                    Cond::Always
                } else {
                    bail!("Line {}: Unsupported condition {}", line_no, cond);
                };
                instructions.push(Instruction::Jump(c, dest, op1, op2));
            } else if tok[0] == "print" {
                instructions.push(Instruction::Print(Rc::new(line[5..].trim().to_string())));
            } else if tok[0] == "printflush" {
                check_n_tok(&tok, 2, line_no)?;
                instructions.push(Instruction::PrintFlush(Rc::new(tok[1].to_string())));
            } else {
                bail!("line {}: unknown instruction {}", line_no, line);
            }
        }

        Ok(Emulator {
            cell,
            instructions,
            vars: HashMap::new(),
            counter: Rc::new(String::from("@counter")),
            watches: Vec::default(),
            breakpoints: Vec::default(),
            print_buffer: Vec::default(),
        })
    }

    /// Runs until `end`, or `n` steps.
    pub fn run(&mut self, max_steps: usize) -> Vec<String> {
        let mut output = Vec::default();

        if self.instructions.is_empty() {
            return output;
        }

        // Ignore breakpoints for the very first step.
        let mut first_step = true;
        while output.len() < max_steps {
            let ip = *self.vars.get(&self.counter).unwrap_or(&0);
            if !first_step && self.breakpoints.contains(&ip) {
                output.push(format!("Hit breakpoint at {}", ip));
                return output;
            }
            first_step = false;

            self.vars.insert(self.counter.clone(), ip + 1);
            let instruction = &self.instructions[ip];
            let watch_output: Vec<_> = self
                .watches
                .iter()
                .map(|n| {
                    if n.starts_with("*") {
                        format!("{}:<not_implemented>", &n)
                    } else {
                        match self.vars.get(n.as_ref()) {
                            Some(v) => format!("{}:{} ", &n, &v),
                            None => format!("{}:null ", &n),
                        }
                    }
                })
                .collect();
            output.push(format!(
                "{}:\t{}\"{}\"",
                ip,
                watch_output.join(""),
                instruction,
            ));

            execute(
                instruction,
                &mut self.cell,
                &mut self.vars,
                &self.counter,
                &mut self.print_buffer,
            );

            if let Instruction::PrintFlush(which) = instruction {
                for line in self.print_buffer.join("").lines() {
                    output.push(format!("\tPrinted to {}: {}", &which, line));
                }
                self.print_buffer.clear();
            }

            if *instruction == Instruction::End
                || *self.vars.get(&self.counter).unwrap_or(&0) >= self.instructions.len()
            {
                self.vars.insert(self.counter.clone(), 0);
                break;
            }

            if *instruction == Instruction::Pause {
                break;
            }
        }

        output
    }

    pub fn set_breakpoints(&mut self, breakpoints: Vec<usize>) {
        self.breakpoints = breakpoints;
    }

    pub fn set_watches(&mut self, watches: Vec<Rc<String>>) {
        self.watches = watches;
    }

    pub fn get_mem(&self, address: usize) -> Option<usize> {
        let data = &self.cell.as_ref()?.data;
        if address >= data.len() {
            None
        } else {
            data[address]
        }
    }

    pub fn get_var(&self, var: &Rc<String>) -> Option<usize> {
        resolve(&self.vars, var)
    }
}

fn check_n_tok(tok: &[&str], n: usize, line_no: usize) -> Result<()> {
    if tok.len() != n {
        bail!("Line {}: {} takes {} arguments", line_no, tok[0], n - 1)
    } else {
        Ok(())
    }
}

fn execute(
    instruction: &Instruction,
    cell: &mut Option<Cell>,
    vars: &mut HashMap<Rc<String>, usize>,
    counter: &Rc<String>,
    print_buffer: &mut Vec<String>,
) {
    match instruction {
        Instruction::End => {}
        Instruction::Pause => {}
        Instruction::Math(math, dest, op1, op2) => {
            let op1 = resolve(vars, op1).unwrap_or(0);
            let op2 = resolve(vars, op2).unwrap_or(0);

            let r = match math {
                Math::Add => op1.overflowing_add(op2).0,
                Math::Sub => op1.overflowing_sub(op2).0,
                Math::Mul => op1.overflowing_mul(op2).0,
                Math::Mod if op2 > 0 => op1 % op2,
                Math::Mod => 0,
            };
            vars.insert(dest.clone(), r);
        }
        Instruction::Read(name, cell_name, address) => {
            let val = match (resolve(vars, address), cell.as_ref()) {
                (Some(address), Some(cell))
                    if cell.name == *cell_name && address < cell.data.len() =>
                {
                    cell.data[address]
                }
                _ => None,
            };

            match val {
                Some(val) => {
                    vars.insert(name.clone(), val.clone());
                }
                None => {
                    vars.remove(name);
                }
            }
        }
        Instruction::Write(value, cell_name, address) => {
            match (resolve(vars, address), resolve(vars, value), cell) {
                (Some(address), value, Some(cell))
                    if cell.name == *cell_name && address < cell.data.len() =>
                {
                    cell.data[address] = value;
                }
                _ => {}
            }
        }
        Instruction::Set(dest, source) => match resolve(vars, source) {
            Some(value) => {
                vars.insert(dest.clone(), value);
            }
            None => {
                vars.remove(dest);
            }
        },
        Instruction::PrintFlush(..) => {}
        Instruction::Print(arg) => {
            if arg.starts_with("\"") && arg.ends_with("\"") && arg.len() >= 2 {
                print_buffer.push(
                    arg[1..arg.len() - 1]
                        .replace("\\n", "\n")
                        .replace("\\t", "\t")
                        .replace("\\\"", "\"")
                        .to_string(),
                )
            } else {
                let v = match resolve(vars, arg) {
                    Some(n) => n.to_string(),
                    None => "null".to_string(),
                };
                print_buffer.push(v);
            }
        }
        Instruction::Jump(cond, dest, op1, op2) => {
            let met = match (cond, resolve(vars, op1), resolve(vars, op2)) {
                (Cond::Always, _, _) => true,
                (Cond::Eq, op1, op2) => op1 == op2,
                (Cond::Ne, op1, op2) => op1 != op2,
                (Cond::Lt, op1, op2) => op1 < op2,
                (Cond::Gt, op1, op2) => op1 > op2,
            };

            if met {
                vars.insert(counter.clone(), *dest);
            }
        }
    }
}

pub fn resolve(vars: &HashMap<Rc<String>, usize>, arg: &Rc<String>) -> Option<usize> {
    match arg.parse::<usize>() {
        Ok(n) => Some(n),
        Err(..) => vars.get(arg).copied(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end() {
        let mut emu = Emulator::new(None, "").unwrap();
        assert_eq!(0, emu.run(10).len());

        let mut emu = Emulator::new(None, "jump 1 always x false\nop add foo 1 2\nend").unwrap();
        assert_eq!(3, emu.run(10).len());

        let mut emu = Emulator::new(None, "end").unwrap();
        assert_eq!(1, emu.run(10).len());
    }

    #[test]
    fn test_math() {
        let x = Rc::new(String::from("x"));
        let y = Rc::new(String::from("y"));

        let mut emu = Emulator::new(None, "op add x 1 2\nop sub y 7 3\nop mul x x y").unwrap();
        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&x), Some(3));
        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&y), Some(4));
        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&x), Some(12));
    }

    #[test]
    fn test_loop() {
        let x = Rc::new(String::from("x"));
        let y = Rc::new(String::from("y"));

        let mut emu = Emulator::new(
            None,
            "set x 0\nset y 1\nop mul y 2 y\nop add x x 1\njump 2 lessThan x 5",
        )
        .unwrap();
        assert_eq!(emu.run(100).len(), 17);
        assert_eq!(emu.get_var(&x), Some(5));
        assert_eq!(emu.get_var(&y), Some(32));
    }

    #[test]
    fn test_loop_infinite() {
        let x = Rc::new(String::from("x"));

        let mut emu =
            Emulator::new(None, "op add x x x\nop add x x 1\njump 0 always x false").unwrap();
        assert_eq!(emu.run(3).len(), 3);
        assert_eq!(emu.get_var(&x), Some(1));
        assert_eq!(emu.run(3).len(), 3);
        assert_eq!(emu.get_var(&x), Some(3));
        assert_eq!(emu.run(3).len(), 3);
        assert_eq!(emu.get_var(&x), Some(7));
        assert_eq!(emu.run(3).len(), 3);
        assert_eq!(emu.get_var(&x), Some(15));
        assert_eq!(emu.run(3).len(), 3);
        assert_eq!(emu.get_var(&x), Some(31));
    }

    #[test]
    fn test_read_counter() {
        let x = Rc::new(String::from("x"));
        let y = Rc::new(String::from("y"));
        let z = Rc::new(String::from("z"));
        let counter = Rc::new(String::from("@counter"));

        let mut emu = Emulator::new(
            None,
            "set x @counter\nop add y 3 @counter\nop sub z 10 @counter\nset y @counter",
        )
        .unwrap();
        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&x), Some(1));
        assert_eq!(emu.get_var(&counter), Some(1));

        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&y), Some(5));
        assert_eq!(emu.get_var(&counter), Some(2));

        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&z), Some(7));
        assert_eq!(emu.get_var(&counter), Some(3));

        // The counter is set to one beyond the number of instructions in the
        // program for the final instruction. The wrap around occurs after the
        // final instruction completes.
        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&y), Some(4));
        assert_eq!(emu.get_var(&counter), Some(0));
    }

    #[test]
    fn test_set_counter() {
        let x = Rc::new(String::from("x"));
        let counter = Rc::new(String::from("@counter"));

        let mut emu = Emulator::new(
            None,
            "op mul @counter 2 3\nend\nset x 1\nend\nset x 2\nend\nset x 3\nend\nset x 4\nend\nset x 5\nend\n",
        )
        .unwrap();
        assert_eq!(emu.run(2).len(), 2);
        assert_eq!(emu.get_var(&x), Some(3));
        assert_eq!(emu.get_var(&counter), Some(7));
    }

    #[test]
    fn test_set() {
        let x = Rc::new(String::from("x"));
        let y = Rc::new(String::from("y"));
        let z = Rc::new(String::from("z"));

        let mut emu = Emulator::new(None, "set x 5\nset y x\nop mul z x y").unwrap();
        assert_eq!(emu.run(10).len(), 3);
        assert_eq!(emu.get_var(&x), Some(5));
        assert_eq!(emu.get_var(&y), Some(5));
        assert_eq!(emu.get_var(&z), Some(25));
    }

    #[test]
    fn test_jump() {
        let mut emu = Emulator::new(None, "set x 5\njump 0 lessThan 5 x").unwrap();
        assert_eq!(emu.run(20).len(), 2);

        let mut emu = Emulator::new(None, "set x 5\njump 0 greaterThan 5 x").unwrap();
        assert_eq!(emu.run(20).len(), 2);

        let mut emu = Emulator::new(None, "set x 5\njump 0 greaterThan 6 x").unwrap();
        assert_eq!(emu.run(20).len(), 20);

        let mut emu = Emulator::new(None, "set x 5\njump 0 lessThan x 6").unwrap();
        assert_eq!(emu.run(20).len(), 20);

        let mut emu = Emulator::new(None, "set x 5\njump 0 equal x 5").unwrap();
        assert_eq!(emu.run(20).len(), 20);

        let mut emu = Emulator::new(None, "set x 5\njump 0 equal 6 x").unwrap();
        assert_eq!(emu.run(20).len(), 2);

        let mut emu = Emulator::new(None, "set x 5\njump 0 notEqual 5 x").unwrap();
        assert_eq!(emu.run(20).len(), 2);

        let mut emu = Emulator::new(None, "set x 5\njump 0 notEqual x 6").unwrap();
        assert_eq!(emu.run(20).len(), 20);

        let mut emu = Emulator::new(None, "jump 0 always x false").unwrap();
        assert_eq!(emu.run(20).len(), 20);
    }

    #[test]
    fn test_read_write() {
        let x = Rc::new(String::from("x"));

        let mut emu =
            Emulator::new(None, "read x bank1 5\nwrite 5 bank1 5\nread x bank1 5").unwrap();
        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&x), None);
        assert_eq!(emu.run(2).len(), 2);
        assert_eq!(emu.get_var(&x), None);

        let cell = Cell {
            name: Rc::new("bank1".to_string()),
            data: vec![None; 512],
        };
        let mut emu = Emulator::new(
            Some(cell.clone()),
            "read x bank1 5\nwrite 5 bank1 5\nread x bank1 5",
        )
        .unwrap();
        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&x), None);
        assert_eq!(emu.run(2).len(), 2);
        assert_eq!(emu.get_var(&x), Some(5));

        let mut emu = Emulator::new(
            Some(cell.clone()),
            "op add x 1 1\nop add x 1 1\nwrite @counter bank1 7\nread x bank1 7",
        )
        .unwrap();
        assert_eq!(emu.run(10).len(), 4);
        assert_eq!(emu.get_var(&x), Some(3));

        let mut emu = Emulator::new(
            Some(cell.clone()),
            "write 7 bank1 0\nop add x x x\nread @counter bank1 0\nset x 1\nend\nset x 2\nend\nset x 3\nend\nset x 4\nend\nset x 5\nend\n",
        )
            .unwrap();
        assert_eq!(emu.run(10).len(), 5);
        assert_eq!(emu.get_var(&x), Some(3));

        let mut emu = Emulator::new(
            Some(cell.clone()),
            "write 7 bank1 512\nread x bank1 512\nwrite 10 bank1 1000\nread x bank1 1000\nread x bank1 33\nwrite 12 bank1 33\nread x bank1 33",
        )
            .unwrap();
        assert_eq!(emu.run(2).len(), 2);
        assert_eq!(emu.get_var(&x), None);
        assert_eq!(emu.run(2).len(), 2);
        assert_eq!(emu.get_var(&x), None);
        assert_eq!(emu.run(1).len(), 1);
        assert_eq!(emu.get_var(&x), None);
        assert_eq!(emu.run(2).len(), 2);
        assert_eq!(emu.get_var(&x), Some(12));
    }

    #[test]
    fn test_out_of_bounds_counter_same_as_end() {
        let x = Rc::new(String::from("x"));
        let y = Rc::new(String::from("y"));

        for program in &[
            "op add x x 1\nset @counter 100\nset y 2",
            "op add x x 1\nset @counter 100\n",
            "op add x x 1\nend\nset y 2",
            "op add x x 1\nend\n",
        ] {
            let mut emu = Emulator::new(None, program).unwrap();
            for _ in 0..10 {
                emu.run(100).len();
            }
            assert_eq!(emu.get_var(&x), Some(10));
            assert_eq!(emu.get_var(&y), None);
        }
    }
}
