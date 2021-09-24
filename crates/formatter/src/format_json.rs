use crate::format_token::{GroupToken, IfBreakToken, LineToken};
use crate::{format_tokens, FormatToken, FormatValue, IndentToken, ListToken};
use serde_json::Value;

impl FormatValue for Value {
	fn format(&self) -> FormatToken {
		match self {
			Value::String(string) => FormatToken::string(format!("\"{}\"", string).as_str()),
			Value::Number(number) => {
				let number = number.as_f64().unwrap();
				FormatToken::f64(number)
			}
			Value::Bool(value) => FormatToken::from(value),
			Value::Object(value) => {
				let properties_list: Vec<FormatToken> = value
					.iter()
					.map(|(key, value)| {
						FormatToken::from(format_tokens![
							format!("\"{}\":", key).as_str(),
							FormatToken::Space,
							value.format(),
						])
					})
					.collect();

				let properties = format_tokens![
					LineToken::soft(),
					ListToken::join(
						format_tokens![",", LineToken::soft_or_space(),],
						properties_list
					),
					IfBreakToken::new(","),
				];

				FormatToken::from(GroupToken::new(format_tokens![
					"{",
					IndentToken::new(properties),
					LineToken::soft(),
					"}",
				]))
			}
			Value::Null => FormatToken::string("null"),
			Value::Array(_) => todo!("Implement array"),
		}
	}
}

pub fn json_to_tokens(content: &str) -> FormatToken {
	let json: Value = serde_json::from_str(content).unwrap();

	json.format()
}

#[cfg(test)]
mod test {
	use crate::{format_tokens, FormatToken, IndentToken};

	use super::json_to_tokens;
	use crate::format_token::{GroupToken, IfBreakToken, LineToken};

	#[test]
	fn tokenize_number() {
		let result = json_to_tokens("6.45");

		assert_eq!(FormatToken::string("6.45"), result);
	}

	#[test]
	fn tokenize_string() {
		let result = json_to_tokens(r#""foo""#);

		assert_eq!(FormatToken::string(r#""foo""#), result);
	}

	#[test]
	fn tokenize_boolean_false() {
		let result = json_to_tokens("false");

		assert_eq!(FormatToken::string("false"), result);
	}

	#[test]
	fn tokenize_boolean_true() {
		let result = json_to_tokens("true");

		assert_eq!(FormatToken::string("true"), result);
	}

	#[test]
	fn tokenize_boolean_null() {
		let result = json_to_tokens("null");

		assert_eq!(FormatToken::string("null"), result);
	}

	#[test]
	fn tokenize_object() {
		let input = r#"{ "foo": "bar", "num": 5 }"#;
		let expected = FormatToken::Group(GroupToken::new(format_tokens![
			"{",
			IndentToken::new(FormatToken::concat(format_tokens![
				LineToken::soft(),
				"\"foo\":",
				FormatToken::Space,
				"\"bar\"",
				",",
				LineToken::soft_or_space(),
				"\"num\":",
				FormatToken::Space,
				"5",
				IfBreakToken::new(FormatToken::string(",")),
			])),
			LineToken::soft(),
			"}",
		]));

		let result = json_to_tokens(input);

		assert_eq!(expected, result);
	}
}
