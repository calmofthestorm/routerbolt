use std::rc::Rc;

use anyhow::{Context, Result};
use yew::prelude::*;

use routerbolt::*;

const DEFAULT_PROGRAM: &str = include_str!("../../example.mf");

enum Msg {
    Compile,
    Annotate,
    EmulatorStep,
    EmulatorReset,
    CodeInput(yew::InputData),
    SetWatches(yew::InputData),
    SetBreakpoints(yew::InputData),
    ConfigureEmulate(yew::InputData),
}

struct EmulatorState {
    emu: Emulator,
    code: Rc<String>,
}

struct Model {
    // `ComponentLink` is like a reference to a component.
    // It can be used to send messages to the component
    watches: Vec<Rc<String>>,
    breakpoints: Vec<usize>,
    link: ComponentLink<Self>,
    input_text: Rc<String>,
    output_text: Rc<String>,
    max_steps_per_click: usize,
    source: Rc<String>,
    code: Rc<String>,
    emulator_output: Rc<String>,
    annotated: Rc<String>,
    emulator: Option<EmulatorState>,
    empty_emulator_cell: Option<Cell>,
}

impl Model {
    fn compile_internal(&mut self) -> Result<()> {
        self.emulator.take();
        self.source = self.input_text.clone();
        let ir = parser::parse(&self.source).context("parse")?;
        self.empty_emulator_cell = match &ir.stack_config {
            StackConfig::Internal(..) => None,
            StackConfig::External(cell_name) => Some(Cell::new(cell_name.clone())),
        };
        let (code, annotated) = generate(&ir).context("generate")?;
        self.code = Rc::new(code.join("\n"));
        self.output_text = self.code.clone();
        self.annotated = Rc::new(annotated.join("\n"));
        Ok(())
    }

    fn step_emulator(&mut self) {
        self.compile();

        self.output_text = self.annotated.clone();

        let state = self.emulator.take();
        let mut state = if state.is_none() || state.as_ref().unwrap().code != self.code {
            let cell = self.empty_emulator_cell.clone();
            self.emulator_output = Rc::new(String::default());
            let emu = match Emulator::new(cell, &self.code.clone()) {
                Err(e) => {
                    self.emulator_output =
                        Rc::new(format!("*** EMULATOR INIT FAILED ***\n{:?}", &e));
                    return;
                }
                Ok(mut emulator) => {
                    self.emulator_output = Rc::new(format!("*** EMULATOR READY ***\n"));
                    emulator.set_watches(self.watches.clone());
                    emulator.set_breakpoints(self.breakpoints.clone());
                    emulator
                }
            };
            EmulatorState {
                emu,
                code: self.code.clone(),
            }
        } else {
            state.unwrap()
        };

        let output_lines = state.emu.run(self.max_steps_per_click);

        self.emulator_output = Rc::new(format!(
            "{}\n{}",
            &self.emulator_output,
            output_lines.join("\n")
        ));

        self.emulator = Some(state);
    }

    fn compile(&mut self) {
        if self.input_text != self.source {
            if let Err(e) = self.compile_internal() {
                let mut code = Vec::default();
                code.push(format!(
                    "*** COMPILE ERROR ***\n{:?}\n\n*** WHEN COMPILING INPUT ***\n\n\n",
                    e
                ));
                code.extend(
                    self.source
                        .lines()
                        .enumerate()
                        .map(|(j, line)| format!("{:5}: {}\n", j, line)),
                );
                self.code = Rc::new(code.join(""));

                self.annotated = self.code.clone();
            }
        }
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let default_program = Rc::new(DEFAULT_PROGRAM.to_string());
        let mut this = Self {
            link,
            input_text: default_program.clone(),
            output_text: Rc::new(String::default()),
            emulator_output: Rc::new(String::default()),
            code: Rc::new(String::default()),
            max_steps_per_click: 7000,
            watches: Vec::default(),
            breakpoints: Vec::default(),
            source: Rc::new(String::default()),
            annotated: Rc::new(String::default()),
            emulator: None,
            empty_emulator_cell: None,
        };

        this.compile();

        this
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Compile => {
                self.compile();
                self.output_text = self.code.clone();
                true
            }
            Msg::Annotate => {
                self.compile();
                self.output_text = self.annotated.clone();
                true
            }
            Msg::EmulatorReset => {
                self.emulator.take();
                self.emulator_output = Rc::new(String::default());
                true
            }
            Msg::SetWatches(data) => {
                self.watches = data
                    .value
                    .split_whitespace()
                    .map(|s| Rc::new(s.to_string()))
                    .collect();
                let watches = self.watches.clone();
                self.emulator
                    .as_mut()
                    .map(|state| state.emu.set_watches(watches));

                false
            }
            Msg::SetBreakpoints(data) => {
                self.breakpoints.clear();
                for token in data.value.split_whitespace() {
                    if let Ok(line_no) = token.parse() {
                        self.breakpoints.push(line_no);
                    }
                }
                let breakpoints = self.breakpoints.clone();
                self.emulator
                    .as_mut()
                    .map(|state| state.emu.set_breakpoints(breakpoints));

                false
            }
            Msg::EmulatorStep => {
                self.step_emulator();
                true
            }
            Msg::CodeInput(data) => {
                self.input_text = Rc::new(data.value);
                false
            }
            Msg::ConfigureEmulate(data) => {
                self.max_steps_per_click = data.value.parse().unwrap_or(1);
                false
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <table>
                <tr>
                <td>
                  <button onclick=self.link.callback(|_| Msg::Compile)>{ "Compile" }</button>
                  <button onclick=self.link.callback(|_| Msg::Annotate)>{ "Annotate" }</button>
                </td>
                </tr>
                <tr>
                <td>
                  <textarea oninput = self.link.callback(|text| Msg::CodeInput(text)) rows = "50" cols="100">{DEFAULT_PROGRAM}</textarea>
                </td>
                <td>
                  <textarea rows = "50" cols="100">{self.output_text.as_str()}</textarea>
                </td>
                </tr>
                <tr>
                  <td>
                    <button onclick=self.link.callback(|_| Msg::EmulatorReset)>{ "[Re]start" }</button>
                    <button onclick=self.link.callback(|_| Msg::EmulatorStep)>{ "Step" }</button>
                    <label>{"num steps"}</label>
                    <input value={self.max_steps_per_click.to_string()} type="text" oninput=self.link.callback(|text| Msg::ConfigureEmulate(text))/>
                    <label>{"watches"}</label>
                    <input type="text" oninput=self.link.callback(|text| Msg::SetWatches(text))/>
                    <label>{"breakpoints"}</label>
                    <input type="text" oninput=self.link.callback(|text| Msg::SetBreakpoints(text))/>
                  </td>
                </tr>
                <tr>
                  <td>
                    <textarea rows = "20" cols="100">{self.emulator_output.as_str()}</textarea>
                  </td>
                </tr>
                </table>
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}
