use rowan::{GreenNode, GreenNodeData, GreenToken, GreenTokenData, NodeOrToken};
use std::ops::Deref;
use syntax::SyntaxNode;

/// Id of the parent node into which a child node should be inserted
/// 0 -> Insert to root
/// 1 -> First inserted node
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct ParentNodeId(usize);

impl ParentNodeId {
	pub fn root() -> Self {
		ParentNodeId::default()
	}
}

pub(crate) struct CSTBuilderSnapshot {
	parents_pos: usize,
	children_pos: usize,
}

/// Helps building a Concrete Syntax Tree in the [Printer]. The builder
/// stores the currently started nodes in the [parents] collection and the children of all
/// started nodes in the [children] collection.
#[derive(Debug, Default)]
pub(crate) struct CSTBuilder {
	/// Stores all started nodes together with the index where in the [children] array the
	/// children of this node start.  
	parents: Vec<(GreenNode, usize)>,

	/// Stores the children of the started nodes.
	children: Vec<NodeOrToken<GreenNode, GreenToken>>,
}

impl CSTBuilder {
	/// Appends a node to the parent specified by the parent id. Finishes all nodes from the
	/// node at the top of [parents] along to the node with the [parent] id.
	pub fn append_node(&mut self, parent: ParentNodeId, node: GreenNode) -> ParentNodeId {
		self.finish_children(parent);

		self.parents.push((node, self.children.len()));
		ParentNodeId(self.parents.len())
	}

	/// Appends a node that should be left as is, without updating its children.
	pub fn append_raw_node(&mut self, parent: ParentNodeId, node: GreenNode) {
		self.finish_children(parent);

		self.children.push(NodeOrToken::Node(node));
	}

	/// Appends a parent as a child to the specified in parent
	pub fn append_token(&mut self, parent: ParentNodeId, token: GreenToken) {
		self.finish_children(parent);
		self.children.push(NodeOrToken::Token(token));
	}

	/// Finishes all currently open nodes and returns the root node
	pub fn root_node(mut self) -> SyntaxNode {
		while let Some((node, first_child)) = self.parents.pop() {
			finish_node(node, first_child, &mut self.children);
		}

		assert_eq!(
			1,
			self.children.len(),
			"The children should have been reduced to the root node only"
		);

		let root = self.children.pop().unwrap();

		match root {
			NodeOrToken::Node(node) => SyntaxNode::new_root(node),
			_ => panic!("The root element must be a node"),
		}
	}

	/// Finishes all the started nodes until we reach the passed parent.
	/// This is needed because the printer only does a pre-order traversal of the tokens, meaning
	/// it can't explicitly call finish node.
	fn finish_children(&mut self, parent: ParentNodeId) {
		assert!(
			parent.0 <= self.parents.len(),
			"Id no longer valid, was finish_node called before?"
		);

		for (node, first_child) in self.parents.drain(parent.0..).rev() {
			finish_node(node, first_child, &mut self.children);
		}
	}

	/// Creates a snapshot that allows "rewinding" the builder to a previous state.
	/// Rewinding is only guaranteed to work if no nodes started before the snapshot
	/// are finished after creating the snapshot.
	pub fn snapshot(&self) -> CSTBuilderSnapshot {
		CSTBuilderSnapshot {
			parents_pos: self.parents.len(),
			children_pos: self.children.len(),
		}
	}

	/// Restores the builder back to the state when the snapshot was taken.
	pub fn restore(&mut self, snapshot: CSTBuilderSnapshot) {
		assert!(self.children.len() >= snapshot.children_pos, "snapshot no longer valid, was it restored before or was it used past the point where the snapshot was taken?");
		assert!(self.parents.len() >= snapshot.parents_pos, "snapshot no longer valid, was it restored before or was it used past the point where the snapshot was taken?");

		self.children.truncate(snapshot.children_pos);
		self.parents.truncate(snapshot.parents_pos);
	}
}

fn finish_node(
	node: GreenNode,
	first_child: usize,
	children: &mut Vec<NodeOrToken<GreenNode, GreenToken>>,
) {
	assert!(children.len() > first_child);

	let merged = with_merged_children(node, children, first_child);
	children.push(NodeOrToken::Node(merged));
}

fn with_merged_children(
	node: GreenNode,
	children: &mut Vec<NodeOrToken<GreenNode, GreenToken>>,
	first_child: usize,
) -> GreenNode {
	let existing_children = node.children();
	let new_children = &children[first_child..];

	// Let's try to reuse the existing node whenever possible. The assumption is that most of the
	// file is properly formatted and, therefore, most nodes don't change.
	if existing_children.len() == new_children.len() {
		let equals = existing_children
			.zip(new_children)
			.all(|(existing, new)| match existing {
				NodeOrToken::Node(existing_node) => match new {
					NodeOrToken::Node(new_node) => nodes_shallow_eq(existing_node, new_node),
					_ => false,
				},
				NodeOrToken::Token(existing_token) => match new {
					NodeOrToken::Token(new_token) => existing_token == new_token.deref(),
					_ => false,
				},
			});

		if equals {
			children.truncate(first_child);
			return node;
		}
	};

	GreenNode::new(node.kind(), children.drain(first_child..))
}

/// Tests if two green nodes are identical (identical kind and children point to the same node data)
fn nodes_shallow_eq(lhs: &GreenNodeData, rhs: &GreenNodeData) -> bool {
	if lhs.kind() != rhs.kind() || lhs.children().len() != rhs.children().len() {
		false
	} else {
		/// Returns a ID (the pointer of the underlining data) of a green node or token for a shallow compare
		/// Copied from rowan NodeCache
		fn element_id(elem: NodeOrToken<&'_ GreenNodeData, &'_ GreenTokenData>) -> *const () {
			match elem {
				NodeOrToken::Node(it) => it as *const GreenNodeData as *const (),
				NodeOrToken::Token(it) => it as *const GreenTokenData as *const (),
			}
		}

		lhs.children()
			.map(element_id)
			.eq(rhs.children().map(element_id))
	}
}

#[cfg(test)]
mod tests {
	use super::nodes_shallow_eq;
	use crate::printer::cst_builder::{CSTBuilder, ParentNodeId};
	use crate::Tokens;
	use rowan::{GreenNode, GreenNodeData, GreenToken, NodeOrToken};
	use std::ops::Deref;
	use syntax::{SyntaxKind, SyntaxNode};

	fn create_node(kind: SyntaxKind) -> GreenNode {
		GreenNode::new(rowan::SyntaxKind(kind.into()), vec![])
	}

	fn create_node_with_children(
		kind: SyntaxKind,
		children: Vec<NodeOrToken<GreenNode, GreenToken>>,
	) -> GreenNode {
		GreenNode::new(rowan::SyntaxKind(kind.into()), children)
	}

	#[test]
	fn nodes_shallow_eq_returns_true_for_the_same_node() {
		let node = create_node(SyntaxKind::ROOT);

		assert!(nodes_shallow_eq(&node, &node));
	}

	#[test]
	fn nodes_shallow_eq_returns_true_for_nodes_containing_same_token() {
		let mut tokens = Tokens::default();

		let token = tokens.string("test");
		let lhs =
			create_node_with_children(SyntaxKind::ROOT, vec![NodeOrToken::Token(token.clone())]);
		let rhs = create_node_with_children(SyntaxKind::ROOT, vec![NodeOrToken::Token(token)]);

		assert!(nodes_shallow_eq(&lhs, &rhs));
	}

	#[test]
	fn nodes_shallow_eq_returns_true_for_nodes_containing_same_node() {
		let mut tokens = Tokens::default();

		let common_node = create_node_with_children(
			SyntaxKind::StringLiteral,
			vec![NodeOrToken::Token(tokens.string("test"))],
		);
		let lhs = create_node_with_children(
			SyntaxKind::ROOT,
			vec![NodeOrToken::Node(common_node.clone())],
		);
		let rhs = create_node_with_children(SyntaxKind::ROOT, vec![NodeOrToken::Node(common_node)]);

		assert!(nodes_shallow_eq(&lhs, &rhs));
	}

	#[test]
	fn nodes_shallow_eq_returns_false_for_nodes_of_different_kind() {
		let node = create_node(SyntaxKind::ROOT);
		let node2 = create_node(SyntaxKind::Program);

		assert!(!nodes_shallow_eq(&node, &node2));
	}

	#[test]
	fn nodes_shallow_eq_returns_false_for_nodes_with_different_content() {
		let mut tokens = Tokens::default();

		let root_lhs = create_node_with_children(
			SyntaxKind::Program,
			vec![NodeOrToken::Node(create_node_with_children(
				SyntaxKind::StringLiteral,
				vec![NodeOrToken::Token(tokens.string("test"))],
			))],
		);

		let root_rhs = create_node_with_children(
			SyntaxKind::Program,
			vec![NodeOrToken::Node(create_node_with_children(
				SyntaxKind::StringLiteral,
				vec![NodeOrToken::Token(tokens.string("test"))],
			))],
		);

		// These nodes have the same shape but point to different green nodes which is why they are not shallow equal
		assert!(!nodes_shallow_eq(&root_lhs, &root_rhs));
	}

	#[test]
	fn builder_constructs_a_tree_with_all_nodes_and_tokens() {
		let mut builder = CSTBuilder::default();
		let mut tokens = Tokens::default();

		let root_position =
			builder.append_node(ParentNodeId::default(), create_node(SyntaxKind::Program));
		let child_position = builder.append_node(root_position, create_node(SyntaxKind::Number));
		builder.append_token(child_position, tokens.get(SyntaxKind::NumberToken, "5"));
		builder.append_token(root_position, tokens.whitespace("\n"));

		let root = builder.root_node();

		let expected = SyntaxNode::new_root(create_node_with_children(
			SyntaxKind::Program,
			vec![
				NodeOrToken::Node(create_node_with_children(
					SyntaxKind::Number,
					vec![NodeOrToken::Token(tokens.get(SyntaxKind::NumberToken, "5"))],
				)),
				NodeOrToken::Token(tokens.whitespace("\n")),
			],
		));
		assert_eq!(
			expected.green(),
			root.green(),
			"Expected trees to match.\nleft: {:#?}\nright: {:#?}",
			expected,
			root
		);
	}

	#[test]
	fn builder_reuses_nodes_and_tokens() {
		// program(
		// 	[1],
		//  'abc'
		// )
		let mut tokens = Tokens::default();

		let array = create_node_with_children(
			SyntaxKind::Array,
			vec![
				NodeOrToken::Token(tokens.left_bracket()),
				NodeOrToken::Token(tokens.get(SyntaxKind::NumberToken, "1")),
				NodeOrToken::Token(tokens.right_bracket()),
			],
		);

		let string = create_node_with_children(
			SyntaxKind::StringLiteral,
			vec![
				NodeOrToken::Token(tokens.single_quote()),
				NodeOrToken::Token(tokens.string("abc")),
				NodeOrToken::Token(tokens.single_quote()),
			],
		);

		let program = create_node_with_children(
			SyntaxKind::Program,
			vec![
				NodeOrToken::Node(array.clone()),
				NodeOrToken::Node(string.clone()),
			],
		);

		let mut builder = CSTBuilder::default();

		let program_position = builder.append_node(ParentNodeId::default(), program.clone());

		let array_position = builder.append_node(program_position, array.clone());
		builder.append_token(array_position, tokens.left_bracket());
		builder.append_token(array_position, tokens.get(SyntaxKind::NumberToken, "1"));
		builder.append_token(array_position, tokens.right_bracket());

		// convert quotes
		let string_position = builder.append_node(program_position, string);
		builder.append_token(string_position, tokens.double_quote());
		builder.append_token(string_position, tokens.string("abc"));
		builder.append_token(string_position, tokens.double_quote());

		let root = builder.root_node();

		let expected_str = create_node_with_children(
			SyntaxKind::StringLiteral,
			vec![
				NodeOrToken::Token(tokens.double_quote()),
				NodeOrToken::Token(tokens.string("abc")),
				NodeOrToken::Token(tokens.double_quote()),
			],
		);

		let expected_program =
			program.replace_child(1 /* string node */, NodeOrToken::Node(expected_str));

		assert_eq!(expected_program.deref(), root.green().deref(),);

		let generated_array = root.first_child().unwrap();

		// The array should still be the same node as in the original program since its formatting hasn't change
		assert_eq!(
			node_data_ptr(&array),
			node_data_ptr(generated_array.green().deref())
		);
	}

	#[test]
	fn builder_rewinds_when_restoring_a_snapshot() {
		let mut builder = CSTBuilder::default();
		let mut tokens = Tokens::default();

		let program_pos =
			builder.append_node(ParentNodeId::root(), create_node(SyntaxKind::Program));
		let snapshot = builder.snapshot();

		let string_pos = builder.append_node(program_pos, create_node(SyntaxKind::StringLiteral));
		builder.append_token(string_pos, tokens.double_quote());
		builder.append_token(
			string_pos,
			tokens.string("a very long string that causes a line break"),
		);

		builder.restore(snapshot);

		let string_pos = builder.append_node(program_pos, create_node(SyntaxKind::StringLiteral));
		builder.append_token(string_pos, tokens.whitespace("\n"));
		builder.append_token(string_pos, tokens.double_quote());
		builder.append_token(
			string_pos,
			tokens.string("a very long string that causes a line break"),
		);
		builder.append_token(string_pos, tokens.double_quote());

		let root = builder.root_node();

		let expected = SyntaxNode::new_root(create_node_with_children(
			SyntaxKind::Program,
			vec![NodeOrToken::Node(create_node_with_children(
				SyntaxKind::StringLiteral,
				vec![
					NodeOrToken::Token(tokens.whitespace("\n")),
					NodeOrToken::Token(tokens.double_quote()),
					NodeOrToken::Token(
						tokens.string("a very long string that causes a line break"),
					),
					NodeOrToken::Token(tokens.double_quote()),
				],
			))],
		));

		assert_eq!(
			root.green(),
			expected.green(),
			"left: {:#?}\nright: {:#?}",
			root,
			expected
		);
	}

	fn node_data_ptr(data: &GreenNodeData) -> *const () {
		data as *const GreenNodeData as *const ()
	}
}
