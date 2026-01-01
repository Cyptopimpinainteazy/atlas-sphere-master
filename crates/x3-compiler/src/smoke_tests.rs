#[cfg(test)]
mod tests {
    #[test]
    fn lexer_tokenizes_basic_program() {
        let src = "let x = 1;";
        let tokens = crate::lexer::tokenize(src).expect("tokenize should succeed");
        assert!(!tokens.is_empty());
    }
}
