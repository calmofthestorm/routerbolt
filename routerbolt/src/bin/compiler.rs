use std::convert::AsRef;
use std::io::Write;

use anyhow::Context;

use routerbolt::*;

fn main_internal() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();

    let (inp, outp) = if args.len() == 3 {
        (&args[1], &args[2])
    } else {
        eprintln!("Usage {} <infile> <outifle>", &args[0]);
        return Ok(());
    };

    // Parse input into series of `Op`, and determine the offset of each
    // instruction so that we can use them in the second pass. This requires
    // knowing how many instructions each will generate.
    let input_text = std::fs::read(&inp).context("read input file")?;
    let input_text = std::str::from_utf8(&input_text).context("decode input as utf8")?;

    let ir = IntermediateRepresentation::parse(input_text).context("parse")?;
    let (output, annotated) = generate(&ir).context("generate")?;

    write_file(outp.as_ref(), &output).context("write output file")?;
    write_file(format!("{}.annotated", &outp).as_ref(), &annotated)
        .context("write annotated file")?;

    Ok(())
}

fn write_file(path: &std::path::Path, lines: &Vec<String>) -> Result<()> {
    let mut fd = std::fs::File::create(path)?;
    for line in lines.iter() {
        fd.write(line.as_ref())?;
        fd.write(b"\n")?;
    }
    Ok(())
}

fn main() {
    if let Err(e) = main_internal().context("main") {
        eprintln!("{:?}", &e);
    }
}
