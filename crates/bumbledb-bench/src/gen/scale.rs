use crate::gen::Scale;

impl Scale {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::S => "S",
            Self::M => "M",
            Self::L => "L",
        }
    }
}
