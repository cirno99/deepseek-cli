use futures::stream::StreamExt;
use pulldown_cmark::{Options, Parser as MarkdownParser};
use pulldown_cmark_mdcat::{
    resources::FileResourceHandler, Environment, Settings, TerminalProgram, TerminalSize, Theme,
};
use reqwest::Response;
use serde_json::Value;
use std::error::Error;
use std::io::{self, stdout, BufWriter, Write};
use syntect::parsing::SyntaxSet;

pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }

    pub async fn render(&self, response: Response) -> Result<String, Box<dyn Error>> {
        // Get the response body as a stream
        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut total_response = String::new();
        print!("Bot: ");
        // Process each chunk of the stream
        while let Some(result) = stream.next().await {
            match result {
                Ok(bytes) => {
                    buffer.extend_from_slice(&bytes);
                    // Convert buffer to a string for easier processing
                    let buffer_str = String::from_utf8_lossy(&buffer);

                    // Split the buffer into lines and process each line
                    for line in buffer_str.lines() {
                        if line.starts_with("data: ") {
                            // Remove the "data: " prefix
                            let json_str = &line[6..]; // Skip "data: " (6 characters)

                            // Check if the line is empty after removing the prefix
                            if json_str.trim().is_empty() {
                                continue; // Skip empty lines
                            }

                            if json_str == "[DONE]" {
                                // 用于存储生成的HTML

                                // 将Markdown解析为HTML
                                pulldown_cmark_mdcat::push_tty(
                                    &Settings {
                                        terminal_capabilities: TerminalProgram::detect()
                                            .capabilities(),
                                        terminal_size: TerminalSize::default(),
                                        syntax_set: &SyntaxSet::load_defaults_newlines(),
                                        theme: Theme::default(),
                                    },
                                    &Environment::for_local_directory(
                                        &tempfile::tempdir()?.path(),
                                    )?,
                                    &FileResourceHandler::new(104_857_600), // TODO: Maybe make this be a DispatchingResourceHandler?
                                    &mut BufWriter::new(stdout()),
                                    MarkdownParser::new_ext(
                                        &total_response.as_str(),
                                        Options::ENABLE_FOOTNOTES
                                            | Options::ENABLE_TABLES
                                            | Options::ENABLE_STRIKETHROUGH,
                                    ),
                                )?;

                                // End of the response stream
                                print!("\n");
                                return Ok(total_response);
                            }

                            // Try to parse the JSON string
                            match serde_json::from_str::<Value>(json_str) {
                                Ok(json) => {
                                    // Successfully parsed a JSON object
                                    if let Some(content) =
                                        json["choices"][0]["delta"]["content"].as_str()
                                    {
                                        io::stdout().flush().unwrap(); // Flush output to ensure immediate printing
                                        total_response.push_str(content);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error parsing JSON: {}", e);
                                }
                            }
                        }
                    }

                    // Clear the buffer after processing
                    buffer.clear();
                }
                Err(e) => {
                    eprintln!("Error reading response stream: {}", e);
                    return Err(Box::new(e));
                }
            }
        }
        Ok(total_response)
    }
}
