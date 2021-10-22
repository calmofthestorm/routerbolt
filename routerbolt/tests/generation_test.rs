use routerbolt::*;
use test_util::*;

#[test]
fn test_passthrough() {
    let output = test_compile("set x 7", use_cell(false, 1));
    assert!(output.starts_with(&["set MF_stack_sz 0".to_string()]),);
}

#[test]
fn test_passthrough_with_empty_stack() {
    let output = test_compile("set x 7;\n", use_cell(false, 0));
    assert_eq!(output, vec!("set x 7".to_string()));
}

#[test]
fn test_stack_gen() {
    // No stack gen if using memory bank.
    let output = test_compile("", use_cell(true, 3));
    assert_eq!(output, vec!("set MF_stack_sz 0".to_string()));

    for text in &["", ""] {
        let output = test_compile(text, use_cell(false, 0));
        assert!(output.is_empty());
    }

    let output = test_compile("", use_cell(false, 1));
    assert_eq!(
        output,
        vec![
            "set MF_stack_sz 0".to_string(),
            "end".to_string(),
            // Push
            "set MF_stack[0] MF_acc".to_string(),
            "op add MF_stack_sz MF_stack_sz 1".to_string(),
            "set @counter MF_resume".to_string(),
            // Pop
            "set MF_acc MF_stack[0]".to_string(),
            "set @counter MF_resume".to_string(),
            // Poke
            "set MF_stack[0] MF_acc".to_string(),
            "set @counter MF_resume".to_string(),
        ]
    );
}
