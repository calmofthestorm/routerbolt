# Design

## Parsing

Parsing is line-based, like Mindustry's own. Despite some sugar for a recursive
language, the design is most similar to a standard assembler.

There is a pre-parsing phase that finds the `stack_config` directive, function
definitions, and let directives, as these are needed to determine the number of
instructions that will be needed in calls to it.

During the main parsing phase, each instruction is parsed, then converted into
zero or more IR instructions, which we count as we go to determine offset for
jumps. Some forward references are resolved during this parsing pass by use of a
scope stack, which tracks opening and closing braces, while others will be
looked up in a table during code generation.

## Code Generation

Code generation is simple translation of each IR instruction into zero or more
Mindustry commands. We generate the code in the same order as the input.

Using an external cell as a stack is straightforward -- `MF_stack_sz` simply
tracks the current size of the stack, which is one greater than the offset to
read/write.

The internal jump table is more complex. We generate a table for push, pop, and
poke. Peek can use the same logic as pop. To do a push or a pop, we save the
return point in `MF_resume`, and compute the jump table address, then jump to
it. The jump table then accesses a global variable `MF_stack[X]` where X is the
depth. The value is then transferred either to or from `MF_acc`, and control is
returned to `MF_resume`. This is similar to `callproc` with only a single depth
of call permitted.

There's no particularly good reason why, but the `push` table updates the stack
pointer, while the `pop` table expects the "caller" to. The latter is because
the "caller" needs the pre-incremented size after the "call", but there is no
reason the grow stack command in "push" could not be moved to the "call" site.

## Testing

Most tests work by running the generated code on a simulator and observing the
expected sequence of tuples (`a`, `b`, `c`). This makes the tests less picky
about the precise details (including the two different stack backends).
