use std::rc::Rc;

use anyhow::Context;

use routerbolt::*;

fn main_internal() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();

    if args.len() < 4 || (args[1] != "stack" && args[1] != "cell") {
        eprintln!(
            "Usage: {} <stack|cell> <size|name> <infile> <max_steps> [watches]",
            &args[0]
        );
        return Ok(());
    }

    let cell = if args[1] == "stack" {
        // StackConfig::Internal(args[2].parse().context("stack size must be integer"))?;
        None
    } else {
        // StackConfig::External(Rc::new(args[2].to_string()));
        Some(Cell::new(Rc::new(args[2].to_string())))
    };

    let inp = &args[3];
    let max_steps: usize = args[4].parse().context("max_steps must be an integer")?;
    let watches: Vec<Rc<String>> = args[5..]
        .iter()
        .map(|w| w.to_string())
        .map(|s| Rc::new(s.to_string()))
        .collect();

    // Parse input into series of `Op`, and determine the offset of each
    // instruction so that we can use them in the second pass. This requires
    // knowing how many instructions each will generate.
    let input_text = std::fs::read(&inp).context("read input file")?;
    let input_text = std::str::from_utf8(&input_text).context("decode input as utf8")?;
    let mut emu = Emulator::new(cell, &input_text).context("init emulator")?;
    emu.set_watches(watches);
    for line in emu.run(max_steps) {
        println!("{}", &line);
    }
    Ok(())
}

fn main() {
    if let Err(e) = main_internal().context("main") {
        eprintln!("{:?}", &e);
    }
}
