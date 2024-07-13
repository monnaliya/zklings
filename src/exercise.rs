use anyhow::Result;
use crossterm::style::{style, StyledContent, Stylize};
use markdown::{mdast::Node, to_mdast, ParseOptions};
use std::{
    fmt::{self, Display, Formatter}, fs, io::{self, Write}, path::{Path, PathBuf}, process::Command
};

use crate::{
    cmd::{run_cmd, CargoCmd, CircomCmd},
    in_official_repo,
    terminal_link::TerminalFileLink,
    DEBUG_PROFILE,
};

/// The initial capacity of the output buffer.
pub const OUTPUT_CAPACITY: usize = 1 << 14;

// Run an exercise binary and append its output to the `output` buffer.
// Compilation must be done before calling this method.
fn run_bin(bin_name: &str, output: &mut Vec<u8>, target_dir: &Path) -> Result<bool> {
    writeln!(output, "{}", "Output".underlined())?;

    // 7 = "/debug/".len()
    let mut bin_path = PathBuf::with_capacity(target_dir.as_os_str().len() + 7 + bin_name.len());
    bin_path.push(target_dir);
    bin_path.push("debug");
    bin_path.push(bin_name);

    let success = run_cmd(Command::new(&bin_path), &bin_path.to_string_lossy(), output)?;

    if !success {
        // This output is important to show the user that something went wrong.
        // Otherwise, calling something like `exit(1)` in an exercise without further output
        // leaves the user confused about why the exercise isn't done yet.
        writeln!(
            output,
            "{}",
            "The exercise didn't run successfully (nonzero exit code)"
                .bold()
                .red(),
        )?;
    }

    Ok(success)
}

/// See `info_file::ExerciseInfo`
pub struct Exercise {
    pub dir: Option<&'static str>,
    pub name: &'static str,
    pub ext: &'static str,
    /// Path of the exercise file starting with the `exercises/` directory.
    pub path: &'static str,
    pub test: bool,
    pub strict_clippy: bool,
    pub hint: String,
    pub done: bool,
}

impl Exercise {
    pub fn terminal_link(&self) -> StyledContent<TerminalFileLink<'_>> {
        style(TerminalFileLink(self.path)).underlined().blue()
    }

    pub fn is_rust(&self) -> bool {
        self.ext == "rs"
    }

    pub fn is_circom(&self) -> bool {
        self.ext == "circom"
    }

    pub fn is_md(&self) -> bool {
        self.ext == "md"
    } 
}

impl Display for Exercise {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.path.fmt(f)
    }
}

pub trait RunnableExercise {
    fn name(&self) -> &str;
    fn strict_clippy(&self) -> bool;
    fn test(&self) -> bool;
    fn is_rust(&self) -> bool;
    fn is_circom(&self) -> bool;
    fn is_md(&self) -> bool;
    fn path(&self) -> String;

    // Compile, check and run the exercise or its solution (depending on `bin_name´).
    // The output is written to the `output` buffer after clearing it.
    fn run(&self, bin_name: &str, output: &mut Vec<u8>, target_dir: &Path) -> Result<bool> {
        output.clear();

        // Developing the official Rustlings.
        let dev = DEBUG_PROFILE && in_official_repo();

        let build_success = CargoCmd {
            subcommand: "build",
            args: &[],
            bin_name,
            description: "cargo build …",
            hide_warnings: false,
            target_dir,
            output,
            dev,
        }
        .run()?;
        if !build_success {
            return Ok(false);
        }

        // Discard the output of `cargo build` because it will be shown again by Clippy.
        output.clear();

        // `--profile test` is required to also check code with `[cfg(test)]`.
        let clippy_args: &[&str] = if self.strict_clippy() {
            &["--profile", "test", "--", "-D", "warnings"]
        } else {
            &["--profile", "test"]
        };
        let clippy_success = CargoCmd {
            subcommand: "clippy",
            args: clippy_args,
            bin_name,
            description: "cargo clippy …",
            hide_warnings: false,
            target_dir,
            output,
            dev,
        }
        .run()?;
        if !clippy_success {
            return Ok(false);
        }

        if !self.test() {
            return run_bin(bin_name, output, target_dir);
        }

        let test_success = CargoCmd {
            subcommand: "test",
            args: &["--", "--color", "always", "--show-output"],
            bin_name,
            description: "cargo test …",
            // Hide warnings because they are shown by Clippy.
            hide_warnings: true,
            target_dir,
            output,
            dev,
        }
        .run()?;

        let run_success = run_bin(bin_name, output, target_dir)?;

        Ok(test_success && run_success)
    }

    /// Function for running Circom exercises
    fn run_circom(&self, output: &mut Vec<u8>) -> Result<bool> {
        // TODO: check this
        let circuit_dir = Path::new("path/to/your/circom/circuits");
        writeln!(output, "{}", "Compiling Circom circuit...".underlined())?;

        let mut compile_cmd = CircomCmd {
            subcommand: "compile",
            args: &["--r1cs", "--wasm", "--sym"],
            circuit_name: self.name(),
            description: "Compiling Circom circuit",
            output,
            circuit_dir,
        };
    
        let compile_success = compile_cmd.run()?;
    
        if !compile_success {
            return Ok(false);
        }

        writeln!(output, "{}", "Generating proof...".underlined())?;

        // Here you would implement the logic to generate a proof
        // This is a placeholder and would need to be expanded based on your specific requirements
        let proof_success = true;

        writeln!(output, "{}", "Verifying proof...".underlined())?;

        // Here you would implement the logic to verify the proof
        // This is a placeholder and would need to be expanded based on your specific requirements
        let verify_success = true;

        Ok(compile_success && proof_success && verify_success)
    }

    fn run_markdown(&self, output: &mut Vec<u8>) -> Result<bool> {
        let content = fs::read_to_string(self.path())?;
        let options = ParseOptions::gfm();
        let ast = to_mdast(&content, &options).unwrap();
        
        let (question, answer) = self.extract_question_and_answer(&ast)?;

        writeln!(output, "{}", question.trim())?;
        print!("Your answer: ");
        io::stdout().flush()?;

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;

        let success = user_input.trim() == answer.trim();
        if success {
            writeln!(output, "Correct!")?;
        } else {
            writeln!(output, "Incorrect. The correct answer was: {}", answer.trim())?;
        }

        Ok(success)
    }

    fn extract_question_and_answer(&self, ast: &Node) -> Result<(String, String)> {
        let mut question = String::new();
        let mut answer = String::new();
        let mut in_question = false;

        if let Node::Root(root) = ast {
            for child in &root.children {
                match child {
                    Node::Heading(heading) if heading.depth == 1 => {
                        in_question = true;
                        for child in &heading.children {
                            if let Node::Text(text) = child {
                                question.push_str(&text.value);
                            }
                        }
                    },
                    Node::Paragraph(para) if in_question => {
                        for child in &para.children {
                            if let Node::Text(text) = child {
                                question.push_str(&text.value);
                            }
                        }
                    },
                    Node::Code(code) => {
                        answer = code.value.clone();
                        break;
                    },
                    _ => {}
                }
            }
        }

        if question.is_empty() || answer.is_empty() {
            anyhow::bail!("Failed to extract question or answer from markdown");
        }

        Ok((question, answer))
    }

    /// Compile, check and run the exercise.
    /// The output is written to the `output` buffer after clearing it.
    #[inline]
    fn run_exercise(&self, output: &mut Vec<u8>, target_dir: &Path) -> Result<bool> {
        if self.is_rust() {
            self.run(self.name(), output, target_dir)
        } else if self.is_circom() {
            self.run_circom(output)
        } else if self.is_md() {
            self.run_markdown(output)
        } else {
            anyhow::bail!("Unsupported exercise type")
        }
    }

    /// Compile, check and run the exercise's solution.
    /// The output is written to the `output` buffer after clearing it.
    fn run_solution(&self, output: &mut Vec<u8>, target_dir: &Path) -> Result<bool> {
        let name = self.name();
        let mut bin_name = String::with_capacity(name.len());
        bin_name.push_str(name);
        bin_name.push_str("_sol");

        self.run(&bin_name, output, target_dir)
    }
}

impl RunnableExercise for Exercise {
    #[inline]
    fn name(&self) -> &str {
        self.name
    }

    #[inline]
    fn path(&self) -> String {
        self.path.to_string()
    }

    #[inline]
    fn strict_clippy(&self) -> bool {
        self.strict_clippy
    }

    #[inline]
    fn test(&self) -> bool {
        self.test
    }

    #[inline]
    fn is_rust(&self) -> bool {
        self.is_rust()
    }

    #[inline]
    fn is_circom(&self) -> bool {
        self.is_circom()
    }

    #[inline]
    fn is_md(&self) -> bool {
        self.is_md()
    }
}
