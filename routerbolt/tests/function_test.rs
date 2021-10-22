use routerbolt::*;
use test_util::*;

/// Basic function call with no return values.
fn basic_function_test_fixture(cell: bool) {
    let text = "set a 1
                call interact
                set c 3
                end

                fn interact {
                  set b 2
                  return;
                }
            ";

    let output = test_compile(text, use_cell(cell, 16));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(1), Some(2), Some(3), 20);
}

#[test]
fn basic_test_function_stack() {
    basic_function_test_fixture(false);
}

#[test]
fn basic_test_function_cell() {
    basic_function_test_fixture(true);
}

/// Function that returns one value
fn basic_return_test_fixture(cell: bool) {
    let text = "set a 1
                call interact -> b
                set c 3
                end

                fn interact -> rv {
                  return 2;
                }
            ";

    let output = test_compile(text, use_cell(cell, 16));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(1), Some(2), Some(3), 40);
}

#[test]
fn basic_test_return_stack() {
    basic_return_test_fixture(false);
}

#[test]
fn basic_test_return_cell() {
    basic_return_test_fixture(true);
}

fn basic_return_multiple_test_fixture(cell: bool) {
    let text = "call interact -> a b c
                end

                fn interact -> rv1 rv2 rv3 {
                  return 1 2 3;
                }
            ";

    let output = test_compile(text, use_cell(cell, 16));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(1), Some(2), Some(3), 40);
}

#[test]
fn basic_test_return_multiple_stack() {
    basic_return_multiple_test_fixture(false);
}

#[test]
fn basic_test_return_multiple_cell() {
    basic_return_multiple_test_fixture(true);
}

/// As above, but multiple nested functions.
fn nested_function_test_fixture(cell: bool) {
    let text = "set a 1
                call f1
                op sub a 10 a
                end

                fn f1 {
                  op add b b 2
                  call f2
                  return;
                }

                fn f2 {
                  op add b b 3
                  call f3
                  return;
                }

                fn f3 {
                  op add b b 5
                  call f4
                  return;
                }

                fn f4 {
                  set c 19
                  set b 1
                  return
                }
            ";

    let output = test_compile(text, use_cell(cell, 16));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(1), Some(10), Some(19), 50);
    step_until_equal(&mut emu, Some(1), Some(1), Some(19), 50);
    step_until_equal(&mut emu, Some(9), Some(1), Some(19), 50);
}

#[test]
fn nested_test_function_stack() {
    nested_function_test_fixture(false);
}

#[test]
fn nested_test_function_cell() {
    nested_function_test_fixture(true);
}

fn recursive_function_test_fixture(cell: bool) {
    // For our purposes, we simply mesaure the divergence between invocations
    // started and completed (and the max stack depth) to verify recursion.
    // Variable tests will exercise this more thoroughly.
    let text = "call rfunc
                end

                fn rfunc {
                  if greaterThan MF_stack_sz c {
                    set c MF_stack_sz
                  }

                  if lessThan a 10 {
                    op add a a 1
                    call rfunc
                    call rfunc
                    op add b b 1
                  }

                  return
                }
            ";

    let output = test_compile(text, use_cell(cell, 64));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(10), None, Some(11), 400);
    step_until_equal(&mut emu, Some(10), Some(1), Some(11), 400);
    step_until_equal(&mut emu, Some(10), Some(10), Some(11), 400);
}

#[test]
fn recursive_test_function_stack() {
    recursive_function_test_fixture(false);
}

#[test]
fn recursive_test_function_cell() {
    recursive_function_test_fixture(true);
}

fn corecursive_function_test_fixture(cell: bool) {
    // This will overflow the stack, but we'll just stop it once it's gone long
    // enough to verify.
    let text = "call f1;
                end;

                fn f1 {
                  if greaterThan MF_stack_sz c {
                    set c MF_stack_sz
                  }

                  op add a a 1
                  call f2
                  return;
                }

                fn f2 {
                  if greaterThan MF_stack_sz c {
                    set c MF_stack_sz
                  }

                  op add b b 1
                  call f1
                  return
                }
            ";

    let output = test_compile(text, use_cell(cell, 100));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(4), Some(4), Some(8), 100);
}

#[test]
fn corecursive_test_function_stack() {
    corecursive_function_test_fixture(false);
}

#[test]
fn corecursive_test_function_cell() {
    corecursive_function_test_fixture(true);
}

fn manual_fibonacci_function_test_fixture(cell: bool) {
    // This uses the stack directly while also calling functions, which is not
    // recommended for general use.
    //
    // `fibonacci` uses global a for the argument, so recursive calls will need to
    // preserve their argument on the stack.
    let text = "set a 9;
                call fibonacci -> b;

                // Uninterested in the final state of `a`, so just set it to be the same
                // as the result to simplify the assertion.
                set a b
                set c 1
                end;

                // The names used for return arguments here are just documentation.
                // The values specified in the return statement will be bound to
                // whatever names were provided by the call. Hence, we use 'result'
                // here, but 'answer' in the body, and 'b' in the outermost call,
                // 'intermediate' within.
                fn fibonacci -> result {
                  if lessThan a 2 {
                    return a
                  }

                  set MF_acc a
                  push
                  op sub a a 1

                  // We are storing the result in a global variable, so the second invocation
                  // below will overwrite it. We'll need to save this on the stack.
                  call fibonacci -> intermediate
                  pop
                  op sub a MF_acc 2

                  set MF_acc intermediate
                  push

                  call fibonacci -> intermediate
                  pop
                  op add answer intermediate MF_acc
                  return answer;
                }
            ";

    let output = test_compile(text, use_cell(cell, 100));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(34), Some(34), Some(1), 4000);
}

#[test]
fn manual_fibonacci_function_test_stack() {
    manual_fibonacci_function_test_fixture(false);
}

#[test]
fn manual_fibonacci_function_test_cell() {
    manual_fibonacci_function_test_fixture(true);
}

fn basic_return_recursive_test_fixture(cell: bool) {
    let text = "call interact -> a b c
                set a 0
                end

                fn interact -> rv1 rv2 rv3 {
                  if lessThan d 10 {
                    op add d d 1
                    call interact -> x y z
                    op add x 1 x
                    op add y 2 y
                    op add z 4 z
                    return x y z
                  } else {
                    return 1 2 3;
                  }
                }";

    let output = test_compile(text, use_cell(cell, 16));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(11), Some(22), Some(43), 1000);
    step_until_equal(&mut emu, Some(0), Some(22), Some(43), 1000);
}

#[test]
fn basic_test_return_recursive_stack() {
    basic_return_recursive_test_fixture(false);
}

#[test]
fn basic_test_return_recursive_cell() {
    basic_return_recursive_test_fixture(true);
}
