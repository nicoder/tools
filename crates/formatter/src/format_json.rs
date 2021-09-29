use crate::format_token::NodeToken;
use crate::{
	format_tokens, FormatContext, FormatToken, FormatValue, GroupToken, LineToken, ListToken,
};
use rowan::GreenNode;
use serde_json::Value;
use syntax::SyntaxKind;

#[inline]
fn create_green_node(kind: SyntaxKind) -> GreenNode {
	GreenNode::new(rowan::SyntaxKind(kind.into()), vec![])
}

impl FormatValue for Value {
	fn format(&self, context: &mut FormatContext) -> FormatToken {
		match self {
			Value::String(string) => string_token(string, context),

			Value::Number(number) => {
				let number = number.as_f64().unwrap();
				let number_token = context
					.tokens
					.get(SyntaxKind::NumberToken, number.to_string().as_str());

				NodeToken::new(create_green_node(SyntaxKind::Number), number_token).into()
			}

			Value::Bool(value) => context
				.tokens
				.get(SyntaxKind::BooleanToken, value.to_string().as_str())
				.into(),

			Value::Object(value) => {
				let separator = format_tokens![context.tokens.comma(), LineToken::soft_or_space(),];

				let properties_list: Vec<FormatToken> = value
					.iter()
					.map(|(key, value)| {
						format_tokens![
							string_token(key, context),
							context.tokens.colon(),
							FormatToken::Space,
							value.format(context),
						]
					})
					.collect();

				let properties = format_tokens![
					LineToken::soft(),
					ListToken::join(separator, properties_list),
				];

				NodeToken::new(
					create_green_node(SyntaxKind::Object),
					GroupToken::new(format_tokens![
						context.tokens.left_brace(),
						FormatToken::indent(properties),
						LineToken::soft(),
						context.tokens.right_brace(),
					]),
				)
				.into()
			}
			Value::Null => context.tokens.null().into(),

			Value::Array(array) => {
				let separator = format_tokens![context.tokens.comma(), LineToken::soft_or_space(),];

				let elements = format_tokens![
					LineToken::soft(),
					ListToken::join(
						separator,
						array.iter().map(|element| element.format(context))
					),
				];

				NodeToken::new(
					create_green_node(SyntaxKind::Array),
					GroupToken::new(format_tokens![
						context.tokens.left_bracket(),
						FormatToken::indent(elements),
						LineToken::soft(),
						context.tokens.right_bracket(),
					]),
				)
				.into()
			}
		}
	}
}

fn string_token(string: &str, context: &mut FormatContext) -> FormatToken {
	let escaped = string
		.replace("\\", "\\\\")
		.replace('"', "\\\"")
		.replace('\r', "\\r")
		.replace('\t', "\\t")
		.replace('\n', "\\n");

	NodeToken::new(
		create_green_node(SyntaxKind::StringLiteral),
		format_tokens!(
			context.tokens.double_quote(),
			context.tokens.string(escaped.as_str()),
			context.tokens.double_quote()
		),
	)
	.into()
}

pub fn json_to_tokens(content: &str) -> FormatToken {
	let json: Value = serde_json::from_str(content).expect("cannot convert json to tokens");
	format_tokens![
		json.format(&mut FormatContext::default()),
		LineToken::hard()
	]
}

#[cfg(test)]
mod test {
	use crate::format_json::create_green_node;
	use crate::{format_tokens, FormatToken, IndentToken, NodeToken, Tokens};
	use syntax::SyntaxKind;

	use super::json_to_tokens;
	use crate::format_token::{GroupToken, LineToken};

	#[test]
	fn tokenize_number() {
		let result = json_to_tokens("6.45");
		let mut tokens = Tokens::default();

		assert_eq!(
			format_tokens![
				NodeToken::new(
					create_green_node(SyntaxKind::Number),
					format_tokens![tokens.get(SyntaxKind::NumberToken, "6.45")]
				),
				LineToken::hard()
			],
			result
		);
	}

	#[test]
	fn tokenize_string() {
		let result = json_to_tokens(r#""foo""#);
		let mut tokens = Tokens::default();

		assert_eq!(
			format_tokens![
				NodeToken::new(
					create_green_node(SyntaxKind::StringLiteral),
					format_tokens![
						tokens.double_quote(),
						tokens.string("foo"),
						tokens.double_quote(),
					]
				),
				LineToken::hard()
			],
			result
		);
	}

	#[test]
	fn tokenize_boolean_false() {
		let result = json_to_tokens("false");
		let mut tokens = Tokens::default();

		assert_eq!(
			format_tokens![
				tokens.get(SyntaxKind::BooleanToken, "false"),
				LineToken::hard()
			],
			result
		);
	}

	#[test]
	fn tokenize_boolean_true() {
		let result = json_to_tokens("true");
		let mut tokens = Tokens::default();

		assert_eq!(
			format_tokens![
				tokens.get(SyntaxKind::BooleanToken, "true"),
				LineToken::hard()
			],
			result
		);
	}

	#[test]
	fn tokenize_boolean_null() {
		let result = json_to_tokens("null");
		let tokens = Tokens::default();

		assert_eq!(format_tokens![tokens.null(), LineToken::hard()], result);
	}

	#[test]
	fn tokenize_object() {
		let input = r#"{ "foo": "bar", "num": 5 }"#;
		let mut tokens = Tokens::default();

		let expected: FormatToken = format_tokens![
			NodeToken::new(
				create_green_node(SyntaxKind::Object),
				GroupToken::new(format_tokens![
					tokens.left_brace(),
					IndentToken::new(format_tokens![
						LineToken::soft(),
						NodeToken::new(
							create_green_node(SyntaxKind::StringLiteral),
							format_tokens![
								tokens.double_quote(),
								tokens.string("foo"),
								tokens.double_quote(),
							]
						),
						tokens.colon(),
						FormatToken::Space,
						NodeToken::new(
							create_green_node(SyntaxKind::StringLiteral),
							format_tokens![
								tokens.double_quote(),
								tokens.string("bar"),
								tokens.double_quote(),
							]
						),
						tokens.comma(),
						FormatToken::Line(LineToken::soft_or_space()),
						NodeToken::new(
							create_green_node(SyntaxKind::StringLiteral),
							format_tokens![
								tokens.double_quote(),
								tokens.string("num"),
								tokens.double_quote(),
							]
						),
						tokens.colon(),
						FormatToken::Space,
						NodeToken::new(
							create_green_node(SyntaxKind::Number),
							tokens.get(SyntaxKind::NumberToken, "5")
						)
					]),
					LineToken::soft(),
					tokens.right_brace(),
				]),
			),
			LineToken::hard()
		];

		let result = json_to_tokens(input);

		assert_eq!(expected, result);
	}

	#[test]
	fn tokenize_array() {
		let input = r#"[ "foo", "bar", 5 ]"#;
		let mut tokens = Tokens::default();

		let expected = format_tokens![
			NodeToken::new(
				create_green_node(SyntaxKind::Array),
				GroupToken::new(format_tokens![
					tokens.left_bracket(),
					IndentToken::new(format_tokens![
						LineToken::soft(),
						NodeToken::new(
							create_green_node(SyntaxKind::StringLiteral),
							format_tokens![
								tokens.double_quote(),
								tokens.string("foo"),
								tokens.double_quote(),
							]
						),
						tokens.comma(),
						LineToken::soft_or_space(),
						NodeToken::new(
							create_green_node(SyntaxKind::StringLiteral),
							format_tokens![
								tokens.double_quote(),
								tokens.string("bar"),
								tokens.double_quote(),
							]
						),
						tokens.comma(),
						LineToken::soft_or_space(),
						NodeToken::new(
							create_green_node(SyntaxKind::Number),
							tokens.get(SyntaxKind::NumberToken, "5")
						),
					]),
					LineToken::soft(),
					tokens.right_bracket()
				])
			),
			LineToken::hard(),
		];

		let result = json_to_tokens(input);

		assert_eq!(expected, result);
	}
}
