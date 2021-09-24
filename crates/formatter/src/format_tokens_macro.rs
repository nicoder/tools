#[macro_export]
macro_rules! format_tokens {
    ( $( $x:expr ),+ $(,)?) => {
        {
            vec![
				$(
					FormatToken::from($x)
				),+
			]
        }
    };
}
