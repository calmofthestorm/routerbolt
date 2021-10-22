use std::rc::Rc;

use routerbolt::*;
use test_util::*;

fn test_if_fixture(cell: bool, branch: bool) {
    let x = Rc::new(String::from("x"));
    let y = Rc::new(String::from("y"));
    let z = Rc::new(String::from("z"));

    let x_term = if branch { 5 } else { 6 };
    let text = format!(
        "set x {}\nstmt:\nif equal x 5 {{\nset y 6\nset z 7\n}}",
        x_term
    );
    let output = test_compile(&text, use_cell(cell, 2));
    let mut emu = Emulator::new(None, &output.join("\n")).unwrap();

    assert!(emu.run(100).len() < 90);
    assert_eq!(emu.get_var(&x), Some(x_term));
    if branch {
        assert_eq!(emu.get_var(&y), Some(6));
        assert_eq!(emu.get_var(&z), Some(7));
    } else {
        assert_eq!(emu.get_var(&y), None);
        assert_eq!(emu.get_var(&z), None);
    }
}

#[test]
fn test_if_stack_false() {
    test_if_fixture(false, false);
}

#[test]
fn test_if_stack_true() {
    test_if_fixture(false, true);
}

#[test]
fn test_if_cell_false() {
    test_if_fixture(true, false);
}

#[test]
fn test_if_cell_true() {
    test_if_fixture(true, true);
}

fn test_if_else_fixture(cell: bool, branch: bool) {
    let x = Rc::new(String::from("x"));
    let y = Rc::new(String::from("y"));
    let z = Rc::new(String::from("z"));

    let x_term = if branch { 5 } else { 6 };
    let text = format!(
        "set x {}\nstmt:\nif equal x 5 {{\nset y 6\nset z 7\n}} else {{\nset y 1\nset z 2\n}}\n",
        x_term
    );
    let output = test_compile(&text, use_cell(cell, 2));
    let mut emu = Emulator::new(None, &output.join("\n")).unwrap();

    assert!(emu.run(100).len() < 90);
    assert_eq!(emu.get_var(&x), Some(x_term));
    if branch {
        assert_eq!(emu.get_var(&y), Some(6));
        assert_eq!(emu.get_var(&z), Some(7));
    } else {
        assert_eq!(emu.get_var(&y), Some(1));
        assert_eq!(emu.get_var(&z), Some(2));
    }
}

#[test]
fn test_if_else_stack_false() {
    test_if_else_fixture(false, false);
}

#[test]
fn test_if_else_stack_true() {
    test_if_else_fixture(false, true);
}

#[test]
fn test_if_else_cell_false() {
    test_if_else_fixture(true, false);
}

#[test]
fn test_if_else_cell_true() {
    test_if_else_fixture(true, true);
}

fn always(c: bool) -> &'static str {
    if c {
        "always true true"
    } else {
        "equal 0 1"
    }
}

fn test_nested_if_else_fixture(cell: bool, outer: bool, inner1: bool, inner2: bool) {
    let x = Rc::new(String::from("x"));
    let y = Rc::new(String::from("y"));
    let z = Rc::new(String::from("z"));

    // Stack size only affects functions; loops/ifs/etc don't create scopes.
    let text = format!(
        "if {} {{
           op add z z z
           if {} {{
             op sub y 10 7
           }}

           if {} {{
             set x 2
           }} else {{
             set x 47
           }}
         }} else {{
           set z 17

           if {} {{
             set y 18
           }} else {{
             set y 19
           }}

           if {} {{
             set x 5
           }}
         }}",
        always(outer),
        always(inner1),
        always(inner2),
        always(inner1),
        always(inner2),
    );
    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(None, &output.join("\n")).unwrap();

    let mut ex = None;
    let mut ey = None;
    let ez;

    if outer {
        ez = Some(0);

        if inner1 {
            ey = Some(10 - 7);
        }

        if inner2 {
            ex = Some(2);
        } else {
            ex = Some(47);
        }
    } else {
        ez = Some(17);

        if inner1 {
            ey = Some(18);
        } else {
            ey = Some(19);
        }

        if inner2 {
            ex = Some(5);
        }
    }

    assert!(emu.run(100).len() < 90);

    assert_eq!(emu.get_var(&x), ex);
    assert_eq!(emu.get_var(&y), ey);
    assert_eq!(emu.get_var(&z), ez);
}

#[test]
fn test_nested_if_else() {
    let tt = &[false, true];
    for cell in tt {
        for outer in tt {
            for inner1 in tt {
                for inner2 in tt {
                    test_nested_if_else_fixture(*cell, *outer, *inner1, *inner2);
                }
            }
        }
    }
}

fn test_empty_if_fixture(cell: bool) {
    let x = Rc::new(String::from("x"));
    let y = Rc::new(String::from("y"));

    let text = "set x 5
                op mul x x 3
                if always {
                }
                set y 6
                op add y 3 y";
    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(None, &output.join("\n")).unwrap();

    assert!(emu.run(100).len() < 90);

    assert_eq!(emu.get_var(&x), Some(15));
    assert_eq!(emu.get_var(&y), Some(9));
}

#[test]
fn test_empty_if_cell() {
    test_empty_if_fixture(true);
}

#[test]
fn test_empty_if_stack() {
    test_empty_if_fixture(false);
}

fn test_empty_if_else_fixture(cell: bool, cond: bool, has_if: bool, has_else: bool) {
    let x = Rc::new(String::from("x"));
    let y = Rc::new(String::from("y"));
    let z = Rc::new(String::from("z"));

    let body1 = if has_if { "set z 3\n" } else { "" };

    let body2 = if has_else { "set z 4\n" } else { "" };

    let text = format!(
        "set x 5
                        op sub z 13 1
                        op mul x x 3
                        if {} {{
                          {}
                        }} else {{
                          {}
                        }}
                        set y 6
                        op add y 3 y",
        always(cond),
        body1,
        body2
    );
    let output = test_compile(&text, use_cell(cell, 0));
    let mut emu = Emulator::new(None, &output.join("\n")).unwrap();

    assert!(emu.run(100).len() < 90);

    assert_eq!(emu.get_var(&x), Some(15));
    assert_eq!(emu.get_var(&y), Some(9));

    let ez = if cond && has_if {
        Some(3)
    } else if !cond && has_else {
        Some(4)
    } else {
        Some(12)
    };

    assert_eq!(emu.get_var(&z), ez);
}

#[test]
fn test_empty_else_if() {
    let tt = &[false, true];
    for cell in tt {
        for outer in tt {
            for inner1 in tt {
                for inner2 in tt {
                    test_empty_if_else_fixture(*cell, *outer, *inner1, *inner2);
                }
            }
        }
    }
}

/// "Integration" test for each condition user since our parsing is ad-hoc that
/// always/never special case works right.
fn dualistic_cosmology_if_fixture(cell: bool) {
    for ag in &["", "0 1", "0 0", "true false", "true true"] {
        let text = format!(
            "if always {} {{
               set a 3
             }}

             if never {} {{
               set b 4
             }}

             if always {} {{
               set c 5
             }} else {{
               set c 6
             }}

             if never {} {{
               op add b b 100
             }} else {{
               op add b b 200
             }}",
            ag, ag, ag, ag
        );
        let output = test_compile(&text, use_cell(cell, 0));
        let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
        step_until_equal(&mut emu, Some(3), Some(200), Some(5), 100);
    }
}

#[test]
fn test_dualistic_cosmology_if_cell() {
    dualistic_cosmology_if_fixture(true);
}

#[test]
fn test_dualistic_cosmology_if_stack() {
    dualistic_cosmology_if_fixture(false);
}

fn direct_variable_if_test_fixture(cell: bool) {
    let text = "call main
                end

                fn main {
                  let *stack1
                  let *stack2
                  let *stack3

                  set *stack1 2
                  set *stack2 3
                  op mul *stack3 2 *stack2

                  if always *stack1 *stack2 {
                    set a 1
                  } else {
                    set a 2
                  }

                  if never {
                    set a 4
                  } else {
                    set a 3
                  }

                  if equal *stack1 2 {
                    set a 5
                  } else {
                    set a 6
                  }

                  if equal 2 *stack2 {
                    set a 8
                  } else {
                    set a 7
                  }

                  if equal *stack1 *stack2 {
                    set a 10
                  } else {
                    set a 9
                  }

                  if equal *stack1 *stack1 {
                    set a 11
                  } else {
                    set a 12
                  }

                  set *stack2 2
                  if equal *stack1 *stack1 {
                    set a 13
                  } else {
                    set a 14
                  }

                  return
                }";

    let output = test_compile(text, use_cell(cell, 10));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    for j in 0..7 {
        step_until_equal(&mut emu, Some(2 * j + 1), None, None, 1000);
    }
}

#[test]
fn direct_variable_if_test_stack() {
    direct_variable_if_test_fixture(false);
}

#[test]
fn direct_variable_if_test_cell() {
    direct_variable_if_test_fixture(true);
}
