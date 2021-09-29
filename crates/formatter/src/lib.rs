//! Rome's official formatter.
//!
//! The crate exposes some API and utilities to implement the formatting logic.
//!
//! The formatter relies on an [IR], which allows to format any kind of data structure.
//!
//! In order to implement the formatting logic, you need to implement the trait [FormatValue] for
//! the data structure you want to format.
//!
//! Let's say, for example that you have a small data structure that represents a key/value data:
//!
//! ```rust,no_test
//! struct KeyValue {
//!     key: String,
//!     value: String
//! }
//! ```
//!
//! Now, we do want to create this IR for the data structure:
//! ```rust
//! use rome_formatter::{format_tokens, format_token, FormatToken, FormatValue, FormatOptions, FormatContext};
//!
//! struct KeyValue {
//!     key: String,
//!     value: String
//! }
//!
//! impl FormatValue for KeyValue {
//!     fn format(&self, context: &mut FormatContext) -> FormatToken {
//!         format_tokens![
//!             context.tokens.string(self.key.as_str()),
//!             FormatToken::Space,
//!             context.tokens.string("=>"),
//!             FormatToken::Space,
//!             context.tokens.string(self.value.as_str())
//!         ]
//!     }
//! }
//!
//! fn my_function() {
//!     let key_value = KeyValue { key: String::from("lorem"), value: String::from("ipsum") };
//!     let mut context = FormatContext::default();
//!     let token = key_value.format(&mut context);
//!     let options = FormatOptions::default();
//!     let result = format_token(&token, options);
//!     assert_eq!(result.root().text(), "lorem => ipsum");
//! }
//!
//! ```
//! [IR]: https://en.wikipedia.org/wiki/Intermediate_representation

mod format_json;
mod format_token;
mod format_tokens_macro;
mod intersperse;
mod printer;
mod token_cache;
mod tokens;

use std::{fs::File, io::Read, str::FromStr};

use crate::format_json::json_to_tokens;
use crate::printer::{PrintResult, Printer};
pub use format_token::{
	FormatToken, GroupToken, IfBreakToken, IndentToken, LineMode, LineToken, ListToken, NodeToken,
	RawNodeToken,
};
use parser::parse_text;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::path::Path;
pub use tokens::Tokens;

#[derive(Default, Debug)]
pub struct FormatContext {
	pub tokens: Tokens,
}

/// This trait should be implemented on each node/value that should have a formatted representation
pub trait FormatValue {
	fn format(&self, context: &mut FormatContext) -> FormatToken;
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum IndentStyle {
	/// Tab
	Tab,
	/// Space, with its quantity
	Space(u8),
}

impl Default for IndentStyle {
	fn default() -> Self {
		Self::Tab
	}
}

impl FromStr for IndentStyle {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"tab" => Ok(Self::Tab),
			"space" => Ok(Self::Space(2)),
			// TODO: replace this error with a diagnostic
			_ => Err("Value not supported for IndentStyle"),
		}
	}
}

#[derive(Debug, Default)]
pub struct FormatOptions {
	/// The indent style
	indent_style: IndentStyle,
}

impl FormatOptions {
	pub fn new(indent_style: IndentStyle) -> Self {
		Self { indent_style }
	}
}
// TODO: implement me + handle errors
/// Main function
pub fn format(path: &Path, options: FormatOptions) {
	println!(
		"Running formatter to:\n- file {:?}\n- with options {:?}",
		path, options.indent_style
	);

	// we assume that file exists
	let mut file = File::open(&path).expect("cannot open the file to format");
	let mut buffer = String::new();
	// we assume we have permissions
	file.read_to_string(&mut buffer)
		.expect("cannot read the file to format");

	let result = match FormatterLanguage::try_from(path) {
		Ok(language) => format_str(buffer.as_str(), language, options),
		Err(err) => {
			println!("Formatting failed with '{:?}'", err);
			return;
		}
	};

	println!("{}", result.root().text());
}

pub fn format_str(
	content: &str,
	language: FormatterLanguage,
	options: FormatOptions,
) -> PrintResult {
	let tokens = match language {
		FormatterLanguage::JSON => json_to_tokens(content),
		FormatterLanguage::TS => {
			let cst = parse_text(content).unwrap();

			FormatToken::RawNode(RawNodeToken::new(cst))
		}
	};

	format_token(&tokens, options)
}

pub fn format_token(token: &FormatToken, options: FormatOptions) -> PrintResult {
	let printer = Printer::new(options);
	printer.print(token)
}

pub enum FormatterLanguage {
	JSON,
	TS,
}

#[derive(Debug, Clone)]
pub enum GuessFormatterLanguageError {
	UnknownExtension,
	MissingExtension,
}

impl TryFrom<&Path> for FormatterLanguage {
	type Error = GuessFormatterLanguageError;

	fn try_from(path: &Path) -> Result<Self, Self::Error> {
		if let Some(extension) = path.extension().unwrap_or(OsStr::new("")).to_str() {
			match extension {
				"json" => Ok(FormatterLanguage::JSON),
				"ts" => Ok(FormatterLanguage::TS),
				_ => Err(GuessFormatterLanguageError::UnknownExtension),
			}
		} else {
			Err(GuessFormatterLanguageError::MissingExtension)
		}
	}
}
