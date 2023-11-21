use std::fmt::{format, Debug};

use regex::Regex;

struct Line {
    elements: Vec<String>,
    indent_level: usize,
    section: Section,
    comment: Option<String>,
}
impl Debug for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} comment:{3:?} i_level: {} -> section:{:?}",
            self.elements, self.indent_level, self.section, self.comment
        )
    }
}

impl Line {
    fn create<T: ToString, Y: Into<Option<String>>>(
        elements: Vec<T>,
        indent_level: usize,
        section: Section,
        comment: Y,
    ) -> Self {
        return Line {
            elements: elements.iter().map(|e| e.to_string()).collect(),
            indent_level,
            section,
            comment: comment.into(),
        };
    }
}

#[derive(Debug, Clone, Copy)]
enum Section {
    Data,
    Code,
    Skip,
    CodeLabel,
}
fn main() {
    // read in a file from args
    let file =
        std::fs::read_to_string(std::env::args().nth(1).expect("File path required")).unwrap();

    let mut result = Vec::new();
    let mut section = Section::Skip;
    let mut current_indent = 0;
    let label = Regex::new(r"^\s*_\w+:\s*;?.*$").unwrap();
    let instruction = Regex::new(r"[^\s]+,?").unwrap();
    let comment = Regex::new(r";.*").unwrap();
    let data_decleration = Regex::new(r"[^\s]+").unwrap();
    for line in file.lines() {
        if line.trim().starts_with([';', '\"']) {
            result.push(Line::create::<String, _>(
                vec![],
                0,
                Section::Skip,
                line.to_string(),
            ));
            continue;
        }
        if line.contains(".data") {
            section = Section::Data;
            result.push(Line::create(vec![line.to_string()], 0, Section::Skip, None));
            current_indent = 1;
            continue;
        } else if line.contains(".code") {
            section = Section::Code;
            result.push(Line {
                elements: vec![line.to_string()],
                indent_level: 0,
                section: Section::Skip,
                comment: None,
            });
            current_indent = 1;
            continue;
        } else if ["PROC", "ENDP", "END main"]
            .iter()
            .any(|e| line.contains(e))
        {
            // skip proc def ending and end
            result.push(Line {
                elements: vec![line.to_string()],
                indent_level: 0,
                section: Section::Skip,
                comment: None,
            });
            continue;
        }

        match section {
            Section::Data => {
                if line.is_empty() {
                    result.push(Line {
                        indent_level: 0,
                        elements: vec![],
                        section,
                        comment: None,
                    });
                } else {
                    let needle;
                    if let Some(comment) = comment.find(line) {
                        needle = comment.start()..;
                        result.push(Line {
                            indent_level: current_indent,
                            elements: data_decleration
                                .find(line)
                                .map(|m| vec![line[m.start()..needle.start].to_string()])
                                .unwrap(),
                            comment: Some(line[needle].to_string()),
                            section,
                        });
                    } else {
                        result.push(Line {
                            indent_level: current_indent,
                            elements: data_decleration
                                .captures_iter(line)
                                .map(|cap| cap.extract::<0>().0.to_string())
                                .collect(),
                            comment: None,
                            section,
                        });
                    }
                }
                continue;
            }
            Section::Skip => {
                result.push(Line {
                    indent_level: current_indent,
                    elements: vec![line.to_string()],
                    section,
                    comment: None,
                });
                continue;
            }
            _ => (),
        }
        // segment
        if label.is_match(line) {
            let needle;
            if let Some(comment) = comment.find(line) {
                needle = comment.start()..;
                result.push(Line {
                    elements: vec![line[..needle.start].trim().to_string()],
                    indent_level: current_indent - 1,
                    section: Section::CodeLabel,
                    comment: Some(line[needle].to_string()),
                });
            } else {
                result.push(Line {
                    elements: vec![line.trim().to_string()],
                    indent_level: current_indent - 1,
                    section: Section::CodeLabel,
                    comment: None,
                })
            }
        } else {
            let needle;
            if let Some(comment) = comment.find(line) {
                needle = comment.start()..;
                result.push(Line::create(
                    instruction
                        .captures_iter(&line[..needle.start])
                        .map(|cap| cap.extract::<0>().0.to_string())
                        .collect(),
                    current_indent,
                    section,
                    line[comment.range()].to_string(),
                ));
            } else {
                result.push(Line {
                    elements: instruction
                        .captures_iter(line)
                        .map(|capture| capture.extract::<0>().0.to_string())
                        .collect(),
                    indent_level: current_indent,
                    section,
                    comment: None,
                });
            }
        }
    }
    // reconstruct
    // println!("{:#?}", &result);
    // in spaces
    const INDENT_SIZE: usize = 2;
    let indent: String = " ".repeat(INDENT_SIZE);

    let code_pad = result
        .iter()
        .filter(|line| matches!(line.section, Section::Code))
        .max_by_key(|&line| line.elements.first().unwrap_or(&String::from("")).len())
        .unwrap()
        .elements
        .first()
        .unwrap()
        .len();

    let data_pad = result
        .iter()
        .filter(|line| matches!(line.section, Section::Data))
        .max_by_key(|&line| line.elements.first().unwrap_or(&String::from("")).len())
        .unwrap()
        .elements
        .first()
        .unwrap()
        .len();
    // align padding with indent size
    let (code_pad, data_pad) = (
        code_pad + code_pad % INDENT_SIZE,
        data_pad + data_pad % INDENT_SIZE,
    );
    let formating = result
        .iter()
        .map(|line| {
            (
                line.elements
                    .iter()
                    .enumerate()
                    .map(|(i, val)| match (i, line.section) {
                        (0, Section::Code) => {
                            format!("{}{}", val, " ".repeat(code_pad - val.len()) + &indent,)
                        }
                        (0, Section::Data) => {
                            format!("{}{}", val, " ".repeat(data_pad - val.len()) + &indent,)
                        }
                        (0, Section::CodeLabel) => val.to_string(),
                        _ => {
                            if val.is_empty() {
                                String::from("")
                            } else {
                                format!("{}{}", val, " ")
                            }
                        }
                    })
                    .collect::<Vec<String>>()
                    .join("")
                    .trim()
                    .to_string(),
                indent.repeat(line.indent_level),
                line,
            )
        })
        .collect::<Vec<(_, _, _)>>();
    let comment_pad = formating
        .iter()
        .filter(|(_, _, e)| e.comment.is_some())
        .map(|(a, b, _)| a.len() + b.len())
        .max()
        .unwrap();
    for (line_format, line_indent, line) in formating.iter() {
        let line_comment = if let Some(comment) = line.comment.clone() {
            if !(matches!(line.section, Section::Skip) || comment.trim().is_empty()) {
                " ".repeat(comment_pad - (line_indent.len() + line_format.len()))
                    .to_string()
                    + &comment
            } else {
                comment
            }
        } else {
            String::new()
        };
        println!("{}{}{}", line_indent, line_format, line_comment);
    }
}
