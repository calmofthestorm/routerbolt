use std::rc::Rc;

use routerbolt::*;
use test_util::*;

enum LoopType {
    While,
    DoWhile,
    Infinite,
}

/// A test that can be done for all three loop types, which also exercises break
/// (including nested break).
fn test_common_loop_fixture(cell: bool, loop_type: LoopType) {
    let inner_body = "op add tmp a b
                      op mod tmp tmp 2
                      if equal tmp 1 {
                        // Intentionally not initialized. null + 1 = 1.
                        op add c c 1
                      }
                      op add b b 1";

    let outer_body = "op sub a a 1
                      op sub b 3 a";

    let zero_end = "// a is already 0.
                    set b 0
                    set c 0";

    let text = match loop_type {
        LoopType::DoWhile => format!(
            "set a 3
                                do {{
                                  do {{
                                    {}
                                  }} while lessThan b 3
                                {}
                                }} while greaterThan a 0
                                {}",
            inner_body, outer_body, zero_end
        ),
        LoopType::While => format!(
            "set a 3
                     while greaterThan a 0 {{
                       while lessThan b 3 {{
                         {}
                       }}
                       {}
                     }}
                     {}",
            inner_body, outer_body, zero_end
        ),
        LoopType::Infinite => format!(
            "set a 3
                     loop {{
                       loop {{
                         {}

                         if greaterThan b 2 {{
                           break
                         }}
                       }}
                       {}

                       if equal a 0 {{
                         break
                       }}
                     }}
                     {}",
            inner_body, outer_body, zero_end
        ),
    };

    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();

    let a = Rc::new(String::from("a"));
    while emu.get_var(&a) == None {
        assert_eq!(emu.run(1).len(), 1);
    }

    // beginning of loops.
    step_until_equal(&mut emu, Some(3), None, None, 20);

    // end of first inner loop iteration
    step_until_equal(&mut emu, Some(3), Some(1), Some(1), 20);

    // end of second inner loop iteration
    step_until_equal(&mut emu, Some(3), Some(2), Some(1), 20);

    // end of third inner loop iteration
    step_until_equal(&mut emu, Some(3), Some(3), Some(2), 20);

    // end of first outer loop iteration
    step_until_equal(&mut emu, Some(2), Some(3), Some(2), 20);
    step_until_equal(&mut emu, Some(2), Some(1), Some(2), 20);

    // repeat the inner loop starting at 1
    step_until_equal(&mut emu, Some(2), Some(1), Some(3), 20);
    step_until_equal(&mut emu, Some(2), Some(2), Some(3), 20);
    step_until_equal(&mut emu, Some(2), Some(3), Some(3), 20);

    // end of second outer loop iteration
    step_until_equal(&mut emu, Some(1), Some(3), Some(3), 20);
    step_until_equal(&mut emu, Some(1), Some(2), Some(3), 20);

    // repeat the inner loop starting at 2
    step_until_equal(&mut emu, Some(1), Some(2), Some(3), 20);
    step_until_equal(&mut emu, Some(1), Some(3), Some(4), 20);

    // end of third outer loop iteration
    step_until_equal(&mut emu, Some(0), Some(3), Some(4), 20);
    step_until_equal(&mut emu, Some(0), Some(3), Some(4), 20);

    // Detect the end.
    step_until_equal(&mut emu, Some(0), Some(0), Some(0), 20);
}

#[test]
fn test_common_do_while_stack() {
    test_common_loop_fixture(false, LoopType::DoWhile);
}

#[test]
fn test_common_do_while_cell() {
    test_common_loop_fixture(true, LoopType::DoWhile);
}

#[test]
fn test_common_infinite_stack() {
    test_common_loop_fixture(false, LoopType::Infinite);
}

#[test]
fn test_common_infinite_cell() {
    test_common_loop_fixture(true, LoopType::Infinite);
}

#[test]
fn test_common_while_stack() {
    test_common_loop_fixture(false, LoopType::While);
}

#[test]
fn test_common_while_cell() {
    test_common_loop_fixture(true, LoopType::While);
}

fn test_common_continue_fixture(cell: bool, loop_type: LoopType) {
    let text = match loop_type {
        LoopType::DoWhile => {
            "do {
               op add a a 1
               op mod tmp a 2
               if equal tmp 1 {
                 continue;
               }
               op add b b 1
             } while lessThan a 10"
        }
        LoopType::While => {
            "while lessThan a 10 {
               op add a a 1
               op mod tmp a 2
               if equal tmp 1 {
                 continue;
               }
               op add b b 1
             }"
        }
        LoopType::Infinite => {
            "loop {
               op add a a 1
               op mod tmp a 2
               if equal tmp 1 {
                 continue
               }
               op add b b 1

               if greaterThan a 9 {
                 break;
               }
             }"
        }
    };

    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(10), Some(5), None, 100);
}

#[test]
fn test_common_continue_do_while_stack() {
    test_common_continue_fixture(false, LoopType::DoWhile);
}

#[test]
fn test_common_continue_do_while_cell() {
    test_common_continue_fixture(true, LoopType::DoWhile);
}

#[test]
fn test_common_continue_infinite_stack() {
    test_common_continue_fixture(false, LoopType::Infinite);
}

#[test]
fn test_common_continue_infinite_cell() {
    test_common_continue_fixture(true, LoopType::Infinite);
}

#[test]
fn test_common_continue_while_stack() {
    test_common_continue_fixture(false, LoopType::While);
}

#[test]
fn test_common_continue_while_cell() {
    test_common_continue_fixture(true, LoopType::While);
}

/// Tests the simple case of loops, to distinguish while/do-while semantics,
/// etc.
fn test_do_while_basic_fixture(cell: bool) {
    let text = "set a 5
                do {
                  op add b b 1
                } while notEqual a 5";

    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(5), Some(1), None, 50);
}

#[test]
fn test_do_while_basic_stack() {
    test_do_while_basic_fixture(true);
}

#[test]
fn test_do_while_basic_cell() {
    test_do_while_basic_fixture(false);
}

/// Tests the simple case of loops, to distinguish while/do-while semantics,
/// etc.
fn test_while_basic_fixture(cell: bool) {
    let text = "while never placeholder placeholder {
                  op add b b 1
                }";

    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, None, None, None, 50);
}

#[test]
fn test_while_basic_stack() {
    test_while_basic_fixture(true);
}

#[test]
fn test_while_basic_cell() {
    test_while_basic_fixture(false);
}

/// Tests the simple case of loops, to distinguish infinite/do-infinite semantics,
/// etc.
fn test_infinite_basic_fixture(cell: bool) {
    let text = "loop {
                  break;
                  op add b b 1
                }";

    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, None, None, None, 50);
}

#[test]
fn test_infinite_basic_stack() {
    test_infinite_basic_fixture(true);
}

#[test]
fn test_infinite_basic_cell() {
    test_infinite_basic_fixture(false);
}

fn test_break_continue_fixture(cell: bool) {
    let text = "loop {
                  op add c c 1
                  continue;
                  break;
                }";

    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, None, None, Some(5), 50);

    let text = "loop {
                  op add c c 1
                  break;
                  continue;
                }
                set b 1";

    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, None, Some(1), Some(5), 50);
}

#[test]
fn test_break_continue_stack() {
    test_break_continue_fixture(true);
}

#[test]
fn test_break_continue_cell() {
    test_break_continue_fixture(false);
}

/// "Integration" test for each condition user since our parsing is ad-hoc that
/// always/never special case works right.
fn dualistic_cosmology_loop_fixture(cell: bool) {
    for ag in &["", "0 1", "0 0", "true false", "true true"] {
        let text = format!(
            "while always {} {{
               op add a a 1

               if equal a 5 {{
                 break
               }}
             }}

             while never {} {{
               op add b b 1
             }}

             do {{
               op add c c 1

               if equal c 7 {{
                 break
               }}
             }} while always {}

             do {{
               op add b b 1
             }} while never {}",
            ag, ag, ag, ag
        );
        let output = test_compile(&text, use_cell(cell, 0));
        let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
        step_until_equal(&mut emu, Some(5), None, Some(7), 150);
    }
}

#[test]
fn test_dualistic_cosmology_loop_cell() {
    dualistic_cosmology_loop_fixture(true);
}

#[test]
fn test_dualistic_cosmology_loop_stack() {
    dualistic_cosmology_loop_fixture(false);
}

fn direct_variable_loop_test_fixture(cell: bool) {
    let text = "call main
                end

                fn main {
                  let *stack1
                  let *stack2
                  let *stack3

                  set *stack1 2
                  set *stack2 3
                  op mul *stack3 2 *stack2

                  set sum 0
                  while greaterThan *stack1 0 {
                    do {
                      op add sum sum *stack1
                      op add sum *stack2 sum
                      op add sum sum *stack3
                      op add *stack3 1 *stack3
                      op sub *stack1 *stack1 1
                      op sub *stack2 *stack2 1
                      set a *stack1
                      set b *stack2
                      set c *stack3
                    } while lessThan 0 *stack2
                  }
                  set c sum
                  return
                }";

    let output = test_compile(text, use_cell(cell, 10));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(1), Some(2), Some(7), 500);
    step_until_equal(&mut emu, Some(0), Some(1), Some(8), 500);
}

#[test]
fn direct_variable_loop_test_stack() {
    direct_variable_loop_test_fixture(false);
}

#[test]
fn direct_variable_loop_test_cell() {
    direct_variable_loop_test_fixture(true);
}
