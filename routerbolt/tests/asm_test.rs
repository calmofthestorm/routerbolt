use std::rc::Rc;

use routerbolt::*;
use test_util::*;

fn test_stack_fixture(cell: bool) {
    let a = Rc::new(String::from("a"));
    let b = Rc::new(String::from("b"));
    let c = Rc::new(String::from("c"));

    let text = "set MF_acc 7
                push
                set MF_acc 8
                push
                set MF_acc 9
                push
                peek 0
                set a MF_acc
                peek 2
                set b MF_acc
                pop
                set c MF_acc
         ";
    let output = test_compile(text, use_cell(cell, 64));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();

    assert!(emu.run(200).len() < 190);
    assert_eq!(emu.get_var(&a), Some(9));
    assert_eq!(emu.get_var(&b), Some(7));
    assert_eq!(emu.get_var(&c), Some(9));
}

#[test]
fn test_stack_stack() {
    test_stack_fixture(false);
}

#[test]
fn test_stack_cell() {
    test_stack_fixture(true);
}

fn test_stack_peek_poke_fixture(cell: bool) {
    let text = "set j 0
                do {
                  set MF_acc 12345
                  push
                  op add j j 1
                } while lessThan j 20

                set c 1
                set j 0
                do {
                  op mul MF_acc 3 j
                  poke j
                  op add j j 1
                } while lessThan j 20

                set c 2
                set j 0
                do {
                  peek j
                  set a MF_acc
                  op add j j 1
                } while lessThan j 20

                set c 3
                set j 0
                do {
                  op sub index 19 j
                  peek index
                  set a MF_acc
                  op add j j 1
                } while lessThan j 20

                set c 4
                set j 0
                do {
                  pop
                  set b MF_acc
                  op add j j 1
                } while lessThan j 20";
    let output = test_compile(text, use_cell(cell, 64));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();
    for j in 0..20 {
        step_until_equal(&mut emu, Some(j * 3), None, Some(2), 2000);
    }
    for j in (0..20).rev() {
        step_until_equal(&mut emu, Some(j * 3), None, Some(3), 2000);
    }
    for j in 0..20 {
        step_until_equal(&mut emu, Some(0), Some(j * 3), Some(4), 2000);
    }
}

#[test]
fn test_stack_peek_poke_stack() {
    test_stack_peek_poke_fixture(false);
}

#[test]
fn test_stack_peek_poke_cell() {
    test_stack_peek_poke_fixture(true);
}

fn test_fibonacci_fixture(cell: bool) {
    let fibs: Vec<_> = (0..10)
        .map(|j| format!("set arg {}\ncallproc fibonacci\nset fib{} result\n", j, j))
        .collect();

    let text = format!(
        "start:
         {}

         end
       fibonacci:
         jump fib_done lessThan arg 2

         // Save the input argument
         set MF_acc arg
         push

         // Call `fibonacci(arg - 1)`.
         op sub arg arg 1
         callproc fibonacci

         // Get back the original argument
         pop
         op sub arg MF_acc 2

         // Save the result from the first call.
         set MF_acc result
         push

         // Call `fibonacci(arg - 2)`.
         callproc fibonacci

         // Recover the result of the first call.
         pop
         // Add the two subresults.
         op add result MF_acc result
         ret
       fib_done:
         set result arg
         ret",
        fibs.join("\n")
    );
    let output = test_compile(&text, use_cell(cell, 64));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();

    assert!(emu.run(10000).len() < 9000);
    let mut fibs = Vec::default();
    for j in 0..10 {
        let f = if j < 2 {
            j
        } else {
            let l = fibs.len();
            fibs[l - 2] + fibs[l - 1]
        };
        fibs.push(f);
    }
    for j in 0..10 {
        let fib = format!("fib{}", j);
        assert_eq!(emu.get_var(&Rc::new(fib)), Some(fibs[j]));
    }
}

#[test]
fn test_fibonacci_stack() {
    test_fibonacci_fixture(false);
}

#[test]
fn test_fibonacci_cell() {
    test_fibonacci_fixture(true);
}

fn test_jump_label_fixture(cell: bool) {
    let a = Rc::new(String::from("a"));
    let b = Rc::new(String::from("b"));
    let c = Rc::new(String::from("c"));

    let text = "  set a 0
                  set b 0
                  set c 0
                label1a:
                label1b:
                  jump label3 lessThan b 3
                  op add a a 1
                label2:
                  op add b b 1
                  op add tmp a b
                  jump label1a lessThan tmp 7
                label3:
                  op mul a 2 a
                  jump label2 lessThan a 3
                label4:
                  op mul b 2 b";
    let output = test_compile(text, use_cell(cell, 0));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();

    assert_eq!(emu.get_var(&a), None);
    assert_eq!(emu.get_var(&b), None);
    assert_eq!(emu.get_var(&c), None);

    // Run prelude and set a and b to zero, then single step.
    while emu.get_var(&b) == None {
        assert_eq!(emu.run(1).len(), 1);
    }

    // set c 0
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&c), Some(0));

    // label1a:
    // label1b:
    // jump label3 lessThan b 3 [taken]
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(0));

    // label3:
    // op mul a 2 a
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(0));

    // jump label2 lessThan a 3 [taken]
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(0));

    // label2:
    // op add b b 1
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(1));

    // op add tmp a b
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(1));

    // jump label1a lessThan tmp 7
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(1));

    // jump label3 lessThan b 3
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(1));

    // op mul a 2 a
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(1));

    // jump label2 lessThan a 3
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(1));

    // op add b b 1
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(2));

    // op add tmp a b
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(2));

    // jump label1a lessThan tmp 7
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(2));

    // jump label3 lessThan b 3
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(2));

    // op mul a 2 a
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(2));

    // jump label2 lessThan a 3
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(2));

    // op add b b 1
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(3));

    // op add tmp a b
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(3));

    // jump label1a lessThan tmp 7
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(3));

    // jump label3 lessThan b 3
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(0));
    assert_eq!(emu.get_var(&b), Some(3));

    // op add a a 1
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(1));
    assert_eq!(emu.get_var(&b), Some(3));

    // label2:
    // op add b b 1
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(1));
    assert_eq!(emu.get_var(&b), Some(4));

    // op add tmp a b
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(1));
    assert_eq!(emu.get_var(&b), Some(4));

    // jump label1a lessThan tmp 7
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(1));
    assert_eq!(emu.get_var(&b), Some(4));

    // jump label3 lessThan b 3
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(1));
    assert_eq!(emu.get_var(&b), Some(4));

    // op add a a 1
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(2));
    assert_eq!(emu.get_var(&b), Some(4));

    // label2:
    // op add b b 1
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(2));
    assert_eq!(emu.get_var(&b), Some(5));

    // op add tmp a b
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(2));
    assert_eq!(emu.get_var(&b), Some(5));

    // jump label1a lessThan tmp 7
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(2));
    assert_eq!(emu.get_var(&b), Some(5));

    // label3:
    // op mul a 2 a
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(4));
    assert_eq!(emu.get_var(&b), Some(5));

    // jump label2 lessThan a 3
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(4));
    assert_eq!(emu.get_var(&b), Some(5));

    // label4:
    // op mul b 2 b";
    assert_eq!(emu.run(1).len(), 1);
    assert_eq!(emu.get_var(&a), Some(4));
    assert_eq!(emu.get_var(&b), Some(10));
}

#[test]
fn test_jump_label_stack() {
    test_jump_label_fixture(false);
}

#[test]
fn test_jump_label_cell() {
    test_jump_label_fixture(true);
}

fn direct_fibonacci_variable_test_fixture(cell: bool) {
    let text = "call main
                end

                fn main {
                  let *index
                  set *index 0
                  do {
                    call fibonacci *index -> a
                    op add *index 1 *index
                  } while lessThan *index 12
                  return
                }

                fn fibonacci *n -> f {
                  let *p_1
                  if lessThan *n 2 {
                    return *n;
                  } else {
                    let *f_1

                    op sub *p_1 *n 1
                    call fibonacci *p_1 -> *f_1

                    op sub p_2 *n 2
                    call fibonacci p_2 -> f_2

                    op add answer *f_1 f_2
                    return answer
                }";

    let output = test_compile(text, use_cell(cell, 65536));
    let mut emu = Emulator::new(emu_cell(cell), &output.join("\n")).unwrap();

    let mut f1 = 0;
    let mut f2 = 0;
    for j in 0..12 {
        let fib = if j == 0 {
            0
        } else if j == 1 {
            1
        } else {
            f1 + f2
        };
        f2 = f1;
        f1 = fib;
        step_until_equal(&mut emu, Some(fib), None, None, 250000);
    }
}

#[test]
fn direct_fibonacci_variable_test_stack() {
    direct_fibonacci_variable_test_fixture(false);
}

#[test]
fn direct_fibonacci_variable_test_cell() {
    direct_fibonacci_variable_test_fixture(true);
}
