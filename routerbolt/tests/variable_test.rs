use routerbolt::*;
use test_util::*;

fn global_argument_test_fixture(cell: bool) {
    let text = "set a 1
                call greet 13
                set c 3
                end

                fn greet *stack_arg {
                  set b *stack_arg
                  return;
                }
            ";

    let output = test_compile(text, use_cell(cell, 16));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(1), Some(13), Some(3), 50);
}

#[test]
fn global_argument_stack() {
    global_argument_test_fixture(false);
}

#[test]
fn global_argument_cell() {
    global_argument_test_fixture(true);
}

fn local_argument_test_fixture(cell: bool) {
    let text = "call main
                end

                fn main {
                  let *var;

                  // not used
                  let *var2;
                  let *var3;

                  set *var 13
                  call greet *var
                  set c 3
                  return
                }

                fn greet *stack_arg {
                  set b *stack_arg
                  return;
                }";

    let output = test_compile(text, use_cell(cell, 16));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, None, Some(13), Some(3), 500);
}

#[test]
fn local_argument_stack() {
    local_argument_test_fixture(false);
}

#[test]
fn local_argument_cell() {
    local_argument_test_fixture(true);
}

fn local_argument_return_test_fixture(cell: bool) {
    let text = "call main
                end

                fn main {
                  // Must be defined before used, but we restrict this as a
                  // convenience to the user rather than any need of the
                  // compiler, since it's a common source of bugs IME.
                  let *var;
                  set *var 13

                  let *var2;

                  call greet *var -> *var2
                  set a *var2;
                  set c 3
                  return
                }

                fn greet *stack_arg -> return_name_var {
                  set b *stack_arg
                  return b;
                }";

    let output = test_compile(text, use_cell(cell, 16));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(13), Some(13), Some(3), 500);
}

#[test]
fn local_argument_return_stack() {
    local_argument_return_test_fixture(false);
}

#[test]
fn local_argument_return_cell() {
    local_argument_return_test_fixture(true);
}

fn mixed_variable_test_fixture(cell: bool) {
    let text = "call main
                end

                fn main {
                  if equal 3 4 {
                    let *var1;
                    let *var2;
                  }

                  set *var1 13
                  set *var2 247

                  set a 4

                  call greet a *var1 a *var1 9 b -> c *var2 *var1
                  set a *var2;
                  set b *var1;

                  op add a a b
                  op add a a c

                  return
                }

                // Remember, the return names are documentation only. We'll
                // accept anything except duplicates as long as it is the correct
                // number.
                fn greet *foo *bar *baz *qux *corge *grault -> *ral *ort *tal {
                  // Eventually I'd like to permit direct access without this
                  // awkwardness around global variables. It's straightforward,
                  // but a bit tedious -- namely, all instructions that use them
                  // become variable length, albeit without any forward
                  // dependencies so it shouldn't require more preprocessing.
                  set foo *foo
                  set bar *bar
                  set baz *baz
                  set qux *qux
                  set corge *corge
                  set grault *grault

                  op add accumulate foo bar
                  op add accumulate accumulate baz
                  op add accumulate accumulate qux
                  op add accumulate accumulate corge
                  op add accumulate accumulate grault

                  op mul ort 5 accumulate
                  op sub tal ort 2

                  let *ral
                  let *tal;
                  let *ort;

                  set *ral accumulate
                  set *ort ort
                  set *tal tal

                  return accumulate *ort tal
                }";

    let output = test_compile(text, use_cell(cell, 16));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(215), Some(213), Some(43), 500);
    step_until_equal(&mut emu, Some(471), Some(213), Some(43), 500);
}

#[test]
fn mixed_variable_return_stack() {
    mixed_variable_test_fixture(false);
}

#[test]
fn mixed_variable_return_cell() {
    mixed_variable_test_fixture(true);
}

fn fibonacci_variable_test_fixture(cell: bool, slow: bool) {
    let text = "call main
                end

                fn main {
                  let *f3;
                  let *f6;
                  let *f9;
                  let *f13;

                  call fibonacci 3 -> *f3
                  call fibonacci 6 -> *f6
                  call fibonacci 9 -> *f9
                  call fibonacci 13 -> *f13

                  let *fibsum
                  let *n

                  // Global variable i not used anywhere else.
                  set i 0

                  // This will actually do 3 iterations, but the test ends after
                  // the second.
                  while lessThan i 3 {
                    set *fibsum 0

                    op mul start i 6
                    op add end start 6
                    op add i i 1
                    set *n start

                    loop {
                      // Using n here conflicts with the global variable used in
                      // the fibonacci function.
                      set n *n
                      if equal n end {
                        break
                      }
                      op add tmp n 1
                      set *n tmp

                      call fibonacci n -> a
                      set fibsum *fibsum
                      op add fibsum fibsum a
                      set *fibsum fibsum
                    }

                    set a *f3
                    set b *f6
                    set c *f9

                    set a *f13

                    set b *fibsum
                  }

                  return
                }

                fn fibonacci *n -> f {
                  set n *n
                  if equal n 0 {
                    return 0;
                  } else {
                    if equal n 1 {
                      return 1;
                    } else {
                      if equal n 2 {
                        return 1;
                      } else {
                        if equal n 4 {
                          return 3
                        } else {
                          let *f_1
                          let *f_2

                          op sub n n 2
                          call fibonacci n -> *f_2

                          // Can't use global n as it has been changed. Need to
                          // get the stack var again.
                          set m *n;
                          op sub m m 1
                          call fibonacci m -> *f_1

                          set f_1 *f_1
                          set f_2 *f_2

                          op add answer f_1 f_2
                          return answer
                        }
                      }
                    }
                  }
                }";

    let output = test_compile(text, use_cell(cell, 32768));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();

    step_until_equal(&mut emu, Some(2), Some(8), Some(34), 500000);
    step_until_equal(&mut emu, Some(233), Some(8), Some(34), 500000);

    // 12 is sum(fib[0..6])
    step_until_equal(&mut emu, Some(233), Some(12), Some(34), 50000);

    if slow {
        // Again, but this time do sum(fib[6..12]) (220)
        step_until_equal(&mut emu, Some(2), Some(8), Some(34), 500000);
        step_until_equal(&mut emu, Some(233), Some(8), Some(34), 500000);
        step_until_equal(&mut emu, Some(233), Some(220), Some(34), 500000);

        // Again, but this time do sum(fib[12..18]) (3948)
        step_until_equal(&mut emu, Some(2), Some(8), Some(34), 500000);
        step_until_equal(&mut emu, Some(233), Some(8), Some(34), 500000);
        step_until_equal(&mut emu, Some(233), Some(3948), Some(34), 500000000);
    }
}

#[test]
fn fibonacci_variable_test_stack() {
    fibonacci_variable_test_fixture(false, false);
}

#[test]
fn fibonacci_variable_test_cell() {
    fibonacci_variable_test_fixture(true, false);
}

#[test]
#[ignore]
fn fibonacci_variable_test_stack_slow() {
    fibonacci_variable_test_fixture(false, true);
}

#[test]
#[ignore]
fn fibonacci_variable_test_cell_slow() {
    fibonacci_variable_test_fixture(true, true);
}

fn direct_variable_op_test_fixture(cell: bool) {
    let text = "call main
                end

                fn main {
                  let *stack1
                  let *stack2
                  let *stack3

                  set *stack1 5
                  set global1 7
                  set *stack2 global1
                  set global2 *stack1
                  set *stack1 *stack1
                  set global1 global1

                  set a *stack1
                  set b global1
                  set c 1

                  set a *stack2
                  set b global2
                  set c 2

                  op add *stack1 *stack1 *stack1
                  set a *stack1
                  op add *stack1 *stack1 13
                  set b *stack1
                  set c 3

                  op add *stack1 131 *stack1
                  set a *stack1
                  op sub *stack3 *stack1 *stack2
                  set b *stack3
                  set c 4

                  op add *stack1 *stack1 *stack2
                  set a *stack1
                  op mul *stack2 *stack1 *stack2
                  set b *stack2
                  op add *stack3 2 3
                  set c *stack3
                }";

    let output = test_compile(text, use_cell(cell, 10));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    step_until_equal(&mut emu, Some(5), Some(7), Some(1), 1000);
    step_until_equal(&mut emu, Some(7), Some(5), Some(2), 1000);
    step_until_equal(&mut emu, Some(10), Some(23), Some(3), 1000);
    step_until_equal(&mut emu, Some(154), Some(147), Some(4), 1000);
    step_until_equal(&mut emu, Some(161), Some(1127), Some(5), 1000);
}

#[test]
fn direct_variable_op_test_stack() {
    direct_variable_op_test_fixture(false);
}

#[test]
fn direct_variable_op_test_cell() {
    direct_variable_op_test_fixture(true);
}
