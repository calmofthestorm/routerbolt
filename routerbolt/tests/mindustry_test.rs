use routerbolt::*;
use test_util::*;

fn test_mindustry_fixture(cell: bool) {
    let text = "set a 3\nop sub a a 1\nprint \"hello\"\nmade_up_single_token_ok\nprintflush message1\ngetlink result 0\nubind @poly";
    let output = test_compile(text, use_cell(cell, 0));

    let start = if cell { 1 } else { 0 };

    // External always inits MZ_stack_sz.
    let output = &output[start..];

    let common: Vec<_> = text.lines().map(|l| l.to_string()).collect();

    assert_eq!(output, &common[..]);

    let output = test_compile("", use_cell(cell, 0));
    assert_eq!(output.len(), start);
}

#[test]
fn test_mindustry_cell() {
    test_mindustry_fixture(true);
}

#[test]
fn test_mindustry_stack() {
    test_mindustry_fixture(false);
}

#[test]
fn test_mindustry_print_set_whitespace() {
    let text = "print \"this is a string with whitespace\"";
    let output = test_compile(text, use_cell(false, 0));
    let common: Vec<_> = text.lines().map(|l| l.to_string()).collect();
    assert_eq!(output, common);

    let text = "set hello \"this is a string with whitespace\"\nprint hello";
    let output = test_compile(text, use_cell(false, 0));
    let common: Vec<_> = text.lines().map(|l| l.to_string()).collect();
    assert_eq!(output, common);
}
