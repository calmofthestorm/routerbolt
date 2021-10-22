use std::convert::AsRef;

use crate::*;

pub fn format_arrow_annotation<E, F>(
    prefix: &str,
    func_name: &FunctionName,
    args: &[E],
    returns: &[F],
    ir_index: usize,
) -> String
where
    E: AsRef<str>,
    F: AsRef<str>,
{
    let mut annotation = String::default();
    for arg in args {
        annotation.push(' ');
        annotation += arg.as_ref();
    }
    if !returns.is_empty() {
        annotation += " ->";
        for arg in returns {
            annotation.push(' ');
            annotation += arg.as_ref();
        }
    }

    format!("{} {} {} @{}", prefix, func_name, annotation, ir_index)
}

pub fn format_return_annotation(return_op: &ReturnOp, instr: usize) -> String {
    let returns_ann = if return_op.values.is_empty() {
        "()".to_string()
    } else if return_op.values.len() == 1 {
        format!("({})", return_op.values[0].as_ref())
    } else {
        let mut s = String::from("(");
        for arg in return_op.values.iter() {
            s.push(' ');
            s += arg.as_ref();
        }
        s
    };
    format!(
        "// Return {}{} @{}",
        &return_op.function, returns_ann, instr
    )
}
