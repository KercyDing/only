use super::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guard {
    pub probe: ProbeCall,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCall {
    pub kind: ProbeKind,
    pub argument: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeKind {
    Os,
    Arch,
    Env,
    Cmd,
}
