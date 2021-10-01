use rowan::WalkEvent;
use syntax::parse;
use syntax::{
	ast::{self, AstNode},
	SyntaxNode,
};

fn main() {
	let src = "const foo = <Foo a={1} b={2}>Hi</Foo>;";

	let tree = parse(src, syntax::Language::Tsx).unwrap();
	for event in tree.preorder() {
		if let WalkEvent::Enter(n) = event {
			log_attr(n);
		}
	}

	dbg!(tree.to_string());
}

fn log_attr(n: SyntaxNode) -> Option<()> {
	let attr = ast::JsxAttribute::cast(n)?;
	dbg!(attr);
	None
}
