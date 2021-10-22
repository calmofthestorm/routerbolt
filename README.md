# Introduction

This is a compiler that generates programs which run on
[Mindustry](https://github.com/Anuken/Mindustry) computers. It can be run via
either a command-line interface or [directly in the browser](https://calmofthestorm.github.io/routerbolt/web/dist/).

There is also a simulator that can run Mindustry programs, however it lacks many
features of the actual in-game language, and is primarily intended to test the
compiler. In practice, the simulator is unlikely to be useful for developing
Mindustry programs in its current form, though it could be improved.

# Features

* Jump labels

    `jump` instructions target a `label:` instead of a line number, removing the
    need to re-number them and permitting non-code lines such as comments and
    whitespace.
    
* Line comments

    Lines which are entirely whitespace or start with `//` are ignored.

* Block conditionals (`if`/`else`)

    Conditionals may be specified using familiar `if...else` syntax (`else if`
    is not supported) instead of the assembly-style `jump` instruction:
    
* Loops

* A stack (optional)

    The stack is disabled by default. Enabling it permits use of stack
    variables, `callproc`, `ret`, `fn`, `return`, `call`, `push`, `pop`, `peek`,
    `poke`.
    
    The stack can be configured to use an external memory cell/bank or a jump
    table in the program itself (slower, but no bank/cell required).
    
* Stack variables

    In `fn` functions, stack-allocated variables (name starts with `*`) may be
    used. Stack variables have function scope, permitting recursion. These
    variables can be mixed freely with Mindustry global variables in several
    expressions (`set`, `op`, `print`, `call`, `return`), and are always used
    for the arguments to `fn` functions.

* Functions

    In addition to the low-level `callproc/ret`, recursive functions may be
    defined with `fn` that will generate the code to pass and return an
    arbitrary number of values.
    
# Command-line interface

Run

```
cargo run --bin compiler -- routerbolt/example.mf out
```

to compile a program. This will produce `out`, containing the actual code, and
`out.annotated`, containing an "annotated" version of the code with more
information on which input led to which output.

To run a program on the simulator:

```
# Usage: simulator <stack|cell> <size|name> <infile> <max_steps> [watches]"

# Use external memory bank to run program out for 1000 steps, printing the
# value of global variable a and myvar at each step:
cargo run --bin simulator -- cell bank1 out 1000 a myvar

# Run the program with no external memory bank. Although you are required to
# specify the stack size, it is currently ignored (in the future it may be
# used to detect stack overflow).
cargo run --bin simulator -- size 32 out 1000
```

# Webapp

The compiler and simulator can be used from a [webapp](https://calmofthestorm.github.io/routerbolt/web/dist/). See [Yew instructions](https://yew.rs/getting-started/build-a-sample-app#run-your-app) for how to start a local server.

# Compiler Caveats

This is early stage code. There is solid test coverage for the language, but I
have not done much in-game testing of other than simple programs. It should be
mature enough to play around with it and see how it works. Bugs and less than
ideal UX likely remain other than the cases I'm aware of.

I wrote this just for fun, and don't know how much I'll continue to work on it
going forward. I'd probably fix simple bugs but am unlikely to make major
changes.

Despite basic structured programming constructs, the language remains line
oriented with rigid syntax based on whitespace splitting. For example, an `else`
statement must take exactly the form `} else {` -- `}else` is not permissible,
nor is putting the braces on different lines. I really should have used a parser
generator, even for such a simple syntax.

`print` and `set` are special cases that can be used with quote-enclosed
strings. The parsing of these is not correct (we just take the rest of the line
as the value) so you can write, e.g., `set a "hello`.

Functions require explicit return on all code paths, which isn't checked at
compile time. It is possible for control to "fall" into/out of a function,
corrupting the stack, since functions may be defined anywhere.

Stack overflow is not detected, except that a program with stack commands will
not compile without a `stack_config` directive. But, e.g., we will compile:

```
stack_config size 1
push
push
```

Generated code involving the stack is minimally optimized, and as a result,
stack variables and functions are very expensive. Using the internal stack would
generate ~10 instructions for a function call/return, so on the microprocessor
you could do ~15 function calls per second if those functions immediately
returned.

We do try to be somewhat efficient within an instruction (e.g., `op add *a *a 1`
just does the math directly on the accumulator), but there is no pass over the
IR to optimize between instructions.

Non-stack code isn't too bad, but could still usually be a bit simpler. In
particular, either reordering code or negating the condition would save a jump.

# Simulator Caveats

The compiler is intended to actually be usable to generate programs that run in
the game. The simulator, by contrast, is a quick library I wrote to test the
compiler. It supports a small subset of what Mindustry does, and care was not
taken to match its semantics except where necessary to test the compiler.

In other words, it works great for control flow, and not much else.

One dirty secret of this project is that the design lets the compiler pass along
most ambiguous cases for Mindustry to deal with however it wants. `set 3 4` then
`return 3`? I don't need to know how Mindustry handles that, I can just pass the
ambiguity on. The simulator can't get away with that so easily.

# Language Reference

In general, it is recommended to pick either the assembly style instructions or
the higher level ones and use only them. It is technically possibly to mix and
match, but very easy to corrupt the stack by doing so.

## Mindustry instructions

Unless otherwise specified, any valid Mindustry instruction can be used as is.
The compiler passes them along as is, so you can write `set 5 6` just like you
could in Mindustry, and we don't even try to figure out what you mean.

### `print`

Print is supplemented with the ability to print a single stack variable and
nothing else in a print statement like so:

```
print *myvar
```

### `set`

Global and stack variables may be mixed freely:

```
set a 5
set *stackvar 7
set b *stackvar
set *sv1 *sv2
```

### `op`

Global and stack variables may be mixed freely for the destination and
arguments.

```
op add a *b 7
op add *a *b *c
op add a *b *c
op add a b c
```

### `jump`

Jump **must** use a label for the jump destination, not a line number:

```
set a 0
myloop:
  print a
  op add a a 1
  jump myloop lessThan a 5
printflush message1
```

## Labels

Labels may be specified by ending the line with a `:`. These are used with
`jump` and `callproc`.

## Conditionals

You can use `if` and `if/else` with blocks of code to simplify branching logic.
`else/if` is not supported, but the expression may be nested to achieve the same
effect.

```
if equal b 7 {
   set c 5
}
```

```
if lessThan a 5 {
  print "a is small"
} else {
  print "a is big"
}
```

```
if equal a 0 {
  print "one"
} else {
  if equal a 1 {
    print "two"
  } else {
    print "many"
  }
}
```

## Loops

Three styles of loops are provided, along with `break` and `continue`.

### `while`

```
set a 0
while lessThan a 10 {
  op add a a 2
}
```

### `do-while`

```
do {
  if equal b 7 {
    continue
  }
  op add a a 1
} while lessThan a 10
```

### `loop`

```
loop {
  op add x x 1
  if greaterThan x 10 {
    break
  }
}
```

### `break`

See [`loop`](#Infinite Loop) for an example.

### `continue`

See [`do-while`](#Do-while) for an example.

Note that continue in a `do-while` loop skips to the condition check, not the
start of the loop. I just checked what C++ did and matched that.

## Low-level stack

Low-level stack commands use global variable `MF_acc` as an "accumulator" to
access the stack. Stack underflow/overflow is undefined behavior.

Mixing these with functions and stack variables, while allowed, is not permitted
(except for `stack_config` which is needed to use either).

### `stack_config`

Configures the stack. Use anywhere in the program, at most once. Two forms are accepted.

In-program jump table:

```
stack_config size 1024
```

or an external memory bank or memory cell:

```
stack_config cell bank1
```

### `push`

Pushes `MF_acc` to the stack:

```
set MF_acc 7
push
```

### `pop`

Pops the top of the stack into `MF_acc`:

```
pop
set a MF_acc
```

### `peek`

Copies the stack entry at specified `depth` into `MF_acc`. `depth` may be
omitted to use the top of the stack:

```
set MF_acc 1
push
set MF_acc 2
push
set MF_acc 3
push

peek
// MF_acc = 3

peek 1
// MF_acc = 2

peek 2
// MF_acc = 1

peek 3
// Undefined behavior.
```

### `poke`

Copies `MF_acc` into the stack entry at specified `depth`. `depth` may be
omitted to use the top of the stack:

```
push
push
push

set MF_acc 1
poke
# Stack is now [null, null, 1]

set MF_acc 3
poke 2
# Stack is now [3, null, 1]

set MF_acc 7
poke 1
# Stack is now [2, 7, 1]

pop
// `MF_acc` is now 1

pop
// `MF_acc` is now 7

pop
// `MF_acc` is now 2
```

### `callproc`

Pushes the current `@counter` onto the stack and jumps to `label`:

```
stack_config cell bank1
callproc main
end

main:
  callproc interact
  ret

greet:
  print "Hello!"
  ret
  
interact:
  callproc greet
  callproc flush
  ret
  
flush:
  printflush message1
  ret
```

```
set num 5
callproc fibonacci
print "fibonacci(5) = "
print num
printflush message1
end

// Compute fibonacci(num) -> num
fibonacci:
  jump fibonacci__recursive greaterThan num 1
  ret

fibonacci__recursive:
  set MF_acc num
  push

  op sub num num 2
  callproc fibonacci
  set MF_acc num
  push

  peek 1
  op sub num MF_acc 1
  callproc fibonacci

  pop
  op add num num MF_acc

  pop

  ret
```

### `ret`

Pops the top of the stack and sets `@counter` to that address.

See [`callproc`] for an example.

## Functions and Stack Variables

Requires that the stack be configured with [`stack_config`](#stack_config).
Mixing these with the low-level stack commands is not recommended.

```
stack_config cell bank1
call hello_world
call greet_user "Terry"
call fibonacci 12 -> n
print "fibonacci(12) = "
print n
printflush message1

fn hello_world {
  print "Hello world!"
  return
}

fn greet_user *name {
  print "Hello "
  print *name
  return
}

// Replacing `*result` with any of the following would not change behavior:
// `*a, *num1, result, a, num1, answer, *answer, *sum, *sum`.
fn add_four *num1 *num2 *num3 *num4 -> *result {
  print "Hello "
  print *name
  return
}

fn fibonacci *n -> *answer {
  let *result

  if lessThan *n 2 {
    return *n
  }
  
  // Have to use a stack var here because the recursive call to `fibonacci`
  // will change global variable `tmp`.
  op sub tmp *n 2
  call fibonacci tmp -> *result
  
  op sub tmp *n 1
  call fibonacci tmp -> tmp
  
  op add tmp *result tmp
  
  return tmp
}
```

### `fn`

Defines a function. The arguments must all be stack variables (start with `*`).
The return values may be written as anything in the `fn` provided that all are
unique and the number provided match all calls and returns for that function.

Explicit return is required on all code paths, and we can't currently catch this
at compile time.

All stack variables are function scope regardless of where in the function the
`let` statement occurs.

### `return`

Returns from the function. May include 0 or more values to return, which must
match the number in the `fn`.

### `call`

Calls the specified function. Must have the same number of arguments and return
values as the `fn` definition. When the call site is itself in a function, both
arguments and return values may mix and match freely global and stack variables.

### `let`

Declares a stack variable in the current function.
