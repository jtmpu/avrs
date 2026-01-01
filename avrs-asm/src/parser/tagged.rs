use miette::SourceSpan;

#[derive(Debug, Clone, PartialEq)]
pub struct Tagged<T> {
    pub kind: T,
    pub span: SourceSpan,
}

impl<T> Tagged<T> {
    pub fn new(kind: T, span: SourceSpan) -> Self {
        Self { kind, span }
    }
}
