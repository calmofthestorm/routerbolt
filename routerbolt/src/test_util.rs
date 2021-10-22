use std::rc::Rc;

use crate::*;

pub fn use_cell(cell: bool, size: usize) -> StackConfig {
    if cell {
        StackConfig::External(Rc::new("bank1".to_string()))
    } else {
        StackConfig::Internal(size)
    }
}

pub fn emu_cell(c: bool) -> Option<Cell> {
    if c {
        Some(Cell::default())
    } else {
        None
    }
}

pub fn step_until_equal(
    emu: &mut Emulator,
    ea: Option<usize>,
    eb: Option<usize>,
    ec: Option<usize>,
    mut limit: usize,
) {
    let a = Rc::new(String::from("a"));
    let b = Rc::new(String::from("b"));
    let c = Rc::new(String::from("c"));

    while limit > 0 && (ea != emu.get_var(&a) || eb != emu.get_var(&b) || ec != emu.get_var(&c)) {
        assert_eq!(emu.run(1).len(), 1);
        limit -= 1;
    }
    assert!(limit > 0);
}

/// Prints compiler input and annotated output to stderr. Since by default Cargo
/// swallows output from passing tests, this should only be written on failures.
pub fn test_compile(text: &str, stack_config: StackConfig) -> Vec<String> {
    let text = match stack_config {
        StackConfig::Internal(size) => {
            format!("stack_config size {}\n{}", size, text)
        }
        StackConfig::External(name) => {
            format!("stack_config cell {}\n{}", name, text)
        }
    };

    eprintln!("\n\n---  BEGIN COMPILER INPUT ---\n\n{}\n", &text);
    eprintln!("\n\n---    END COMPILER INPUT ---\n\n");

    let ir = parser::parse(&text).unwrap();
    let (output, annotated) = ir.generate().unwrap();
    eprintln!("\n\n--- BEGIN COMPILER OUTPUT ---\n\n");
    for a in annotated {
        // By default, Rust will only show this listing if the test fails.
        // Convenient for debugging to see the generated code.
        eprintln!("\t{}", a);
    }
    eprintln!("\n\n---   END COMPILER OUTPUT ---\n\n");
    output
}
