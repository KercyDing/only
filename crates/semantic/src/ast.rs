use smol_str::SmolStr;
use text_size::TextRange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentAst {
    pub directives: Vec<DirectiveAst>,
    pub namespaces: Vec<NamespaceAst>,
    pub tasks: Vec<TaskAst>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveAst {
    Verbose { value: bool, range: TextRange },
    Shell { shell: SmolStr, range: TextRange },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceAst {
    pub name: SmolStr,
    pub doc: Option<SmolStr>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAst {
    pub name: SmolStr,
    pub namespace: Option<SmolStr>,
    pub doc: Option<SmolStr>,
    pub params: Vec<ParamAst>,
    pub guard: Option<GuardAst>,
    pub dependencies: Vec<DependencyAst>,
    pub shell: Option<SmolStr>,
    pub shell_fallback: bool,
    pub commands: Vec<CommandAst>,
    pub range: TextRange,
}

impl TaskAst {
    pub fn qualified_name(&self) -> SmolStr {
        match &self.namespace {
            Some(namespace) => SmolStr::from(format!("{namespace}.{}", self.name)),
            None => self.name.clone(),
        }
    }

    pub fn signature(&self) -> SmolStr {
        let mut signature = self.name.to_string();
        signature.push('(');
        signature.push_str(
            &self
                .params
                .iter()
                .map(|parameter| match &parameter.default_value {
                    Some(default) => format!("{}=\"{default}\"", parameter.name),
                    None => parameter.name.to_string(),
                })
                .collect::<Vec<_>>()
                .join(", "),
        );
        signature.push(')');
        SmolStr::from(signature)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamAst {
    pub name: SmolStr,
    pub default_value: Option<SmolStr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardAst {
    pub kind: SmolStr,
    pub argument: SmolStr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyAst {
    pub name: SmolStr,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAst {
    pub text: SmolStr,
    pub interpolations: Vec<InterpolationAst>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpolationAst {
    pub name: SmolStr,
    pub range: TextRange,
}
