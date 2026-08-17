//! Typed definitions for the built-in Only language vocabulary.
//!
//! Parsers and higher layers keep unknown names in the `Unknown` variants so
//! diagnostics can report the original spelling without duplicating string
//! tables across the workspace.

use std::fmt;

use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DirectiveKind {
    Version,
    Var,
    Shell,
    Unknown(SmolStr),
}

impl DirectiveKind {
    pub const SUPPORTED: &'static [Self] = &[Self::Version, Self::Var, Self::Shell];

    pub fn parse(name: &str) -> Self {
        match name {
            "version" => Self::Version,
            "var" => Self::Var,
            "shell" => Self::Shell,
            unknown => Self::Unknown(SmolStr::new(unknown)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Version => "version",
            Self::Var => "var",
            Self::Shell => "shell",
            Self::Unknown(name) => name,
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    pub fn description(&self) -> Option<&'static str> {
        match self {
            Self::Version => {
                Some("Declares the minimum Onlyfile language capability required by this file.")
            }
            Self::Var => Some("Defines a global string value."),
            Self::Shell => Some("Sets the default shell used for task commands."),
            Self::Unknown(_) => None,
        }
    }
}

impl fmt::Display for DirectiveKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GuardKind {
    Os,
    Arch,
    Env,
    Has,
    Unknown(SmolStr),
}

impl GuardKind {
    pub const SUPPORTED: &'static [Self] = &[Self::Os, Self::Arch, Self::Env, Self::Has];

    pub fn parse(name: &str) -> Self {
        match name {
            "os" => Self::Os,
            "arch" => Self::Arch,
            "env" => Self::Env,
            "has" => Self::Has,
            unknown => Self::Unknown(SmolStr::new(unknown)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Os => "os",
            Self::Arch => "arch",
            Self::Env => "env",
            Self::Has => "has",
            Self::Unknown(name) => name,
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    pub fn description(&self) -> Option<&'static str> {
        match self {
            Self::Os => Some("Checks the current operating system."),
            Self::Arch => Some("Checks the current CPU architecture."),
            Self::Env => Some("Checks whether an environment variable exists."),
            Self::Has => Some("Checks whether a command is available."),
            Self::Unknown(_) => None,
        }
    }
}

impl fmt::Display for GuardKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellKind {
    Deno,
    Bash,
    Sh,
    Pwsh,
    Powershell,
    Unknown(SmolStr),
}

impl ShellKind {
    pub const SUPPORTED: &'static [Self] = &[
        Self::Deno,
        Self::Bash,
        Self::Sh,
        Self::Pwsh,
        Self::Powershell,
    ];

    pub fn parse(name: &str) -> Self {
        match name {
            "deno" => Self::Deno,
            "bash" => Self::Bash,
            "sh" => Self::Sh,
            "pwsh" => Self::Pwsh,
            "powershell" => Self::Powershell,
            unknown => Self::Unknown(SmolStr::new(unknown)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Deno => "deno",
            Self::Bash => "bash",
            Self::Sh => "sh",
            Self::Pwsh => "pwsh",
            Self::Powershell => "powershell",
            Self::Unknown(name) => name,
        }
    }

    pub fn fallback(&self) -> Option<Self> {
        match self {
            Self::Pwsh => Some(Self::Powershell),
            Self::Bash => Some(Self::Sh),
            _ => None,
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    pub fn description(&self) -> Option<&'static str> {
        match self {
            Self::Deno => Some("Runs commands with Deno."),
            Self::Bash => Some("Runs commands with Bash."),
            Self::Sh => Some("Runs commands with the system sh."),
            Self::Pwsh => Some("Runs commands with PowerShell Core."),
            Self::Powershell => Some("Runs commands with Windows PowerShell."),
            Self::Unknown(_) => None,
        }
    }
}

impl fmt::Display for ShellKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellOperator {
    Required,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSelection {
    pub kind: ShellKind,
    pub operator: ShellOperator,
}

impl ShellSelection {
    pub fn required(kind: ShellKind) -> Self {
        Self {
            kind,
            operator: ShellOperator::Required,
        }
    }
}

impl ShellOperator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "shell=",
            Self::Fallback => "shell~=",
        }
    }
}

impl fmt::Display for ShellOperator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskShellRef {
    pub selection: ShellSelection,
    pub range: text_size::TextRange,
}
