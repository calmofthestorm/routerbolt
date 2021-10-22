# Retrospective

Overall I was surprised at how quickly this came together. Mindustry's code
model, while frustrating to actually code in, works well as an output
instruction set.

The design conflates the concept of parsing, lexing, and IR more than it should.
Earlier designs used one IR op per line, which proved challenging when I decided
to permit the use of stack variables in more places (initially it was just
function calls and a specialised `lset`/`lget` which have since been combined
with `set`).

By far the most painful part of this project was the parsing. I initially chose
to hand-code the parser because the language was so simple, and don't have
experience with any Rust parser generators yet. This worked well for the
assembly style instructions, but proved more limiting when I decided to add
structured programming concepts, which lend themselves better to a recursive
model.

Handling } is done with a simple stack, but needing to introduce complexity like
that put me off of adding new sugar (for loops, && and || in conditionals, etc),
even though the IR and code gen aspects would be easy. If I'd used a parser
generator from the start, it would have been much easier to add these cases.

Overall, I think that investing the time in the beginning to learn to use a Rust
parser generator would have paid off even for a quick project like this, as well
as providing ongoing benefits from the knowledge gained.

By contrast, the actual code generation proved much easier than I'd expected.
One consequence of the simple line-oriented parser was that rather than parsing
the program into a recursive AST, we just have a list. That would prove limiting
to a more complex language or optimizations, but also simplified some other
aspects. Treating a loop start and end as two different instructions, rather
than a matching pair, worked well for this simple case.

Overall, I think that a recursive AST vs this linear model was a toss-up for a
language of this complexity. Anything more complicated and I'd want a real AST
to operate on when generating IR, but I'm surprised how easy it was to map the
basics of structured programming onto an assembler-like linear format with
limited use of forward references.

The current design is good for what it does, but would complicate many further
refinements such as optimization and return-flow analysis.

I was also surprised at how well my quick-and-dirty emulator matched the game
for the cases I care about (control flow). After refactoring the whole compiler,
I loaded fibonacci into the game and it just worked, both with internal and
external stack. Whether luck or skill, I spent very little time fighting bugs in
the game itself, as most were found and fixed via the emulator-based tests.

Using an emulator to test rather than checking generated code was also a good
choice. Testing that variables take on expected values in sequence abstracts
away the exact instructions used, number of them, etc.

The webapp was also surprisingly straightforward for this use case. I wanted a
webapp as a convenient way for those on their phones to tweak programs. I like
doing it all client side, and Yew worked wonderfully for this case. I'd expect
that, given the compiler only has a single direct dependency, but it was still
nice how quickly I was able to get something basic, if not beautiful, working
and deployed.

Now I'm finally ready to play Debris Field!
